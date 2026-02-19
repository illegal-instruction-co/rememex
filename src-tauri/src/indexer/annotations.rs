use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use arrow_array::{
    Float32Array, FixedSizeListArray, Int64Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::Table;
use log::{debug, info};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::state::ProviderState;

#[derive(Serialize, Clone, Debug)]
pub struct Annotation {
    pub id: String,
    pub path: String,
    pub note: String,
    pub source: String,
    pub created_at: i64,
}

fn annotations_table_name(container_table: &str) -> String {
    format!("{}_annotations", container_table)
}

fn make_annotations_schema(dim: usize) -> Schema {
    Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("path", DataType::Utf8, false),
        Field::new("note", DataType::Utf8, false),
        Field::new("source", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                dim as i32,
            ),
            false,
        ),
        Field::new("created_at", DataType::Int64, false),
    ])
}

fn generate_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("ann_{}", ts)
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

async fn get_or_create_annotations_table(
    db: &Connection,
    container_table: &str,
    dim: usize,
) -> Result<Table> {
    let table_name = annotations_table_name(container_table);

    if let Ok(table) = db.open_table(&table_name).execute().await {
        let schema = table.schema().await?;
        if schema.column_with_name("source").is_some() {
            return Ok(table);
        }
        log::warn!("Annotations table '{}' missing 'source' column, recreating", table_name);
        db.drop_table(&table_name, &[]).await?;
    }

    let schema = Arc::new(make_annotations_schema(dim));
    let table = db
        .create_table(&table_name, RecordBatchIterator::new(vec![], schema))
        .execute()
        .await?;

    info!("Annotations table '{}' created (dim={})", table_name, dim);
    Ok(table)
}

pub async fn add_annotation(
    db: &Connection,
    container_table: &str,
    provider_state: &Arc<Mutex<ProviderState>>,
    path: &str,
    note: &str,
    source: &str,
) -> Result<Annotation> {
    let vector = {
        let guard = provider_state.lock().await;
        let provider = guard
            .provider
            .as_ref()
            .ok_or_else(|| anyhow!("Embedding provider not initialized"))?;
        let vectors: Vec<Vec<f32>> = provider.embed_passages(vec![note.to_string()]).await?;
        vectors.into_iter().next().ok_or_else(|| anyhow!("Empty embedding result"))?
    };

    let dim = vector.len();
    let table = get_or_create_annotations_table(db, container_table, dim).await?;

    let id = generate_id();
    let created_at = now_unix();

    let schema = Arc::new(make_annotations_schema(dim));
    let vector_array = FixedSizeListArray::try_new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        dim as i32,
        Arc::new(Float32Array::from(vector)),
        None,
    )?;

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(vec![id.as_str()])),
            Arc::new(StringArray::from(vec![path])),
            Arc::new(StringArray::from(vec![note])),
            Arc::new(StringArray::from(vec![source])),
            Arc::new(vector_array),
            Arc::new(Int64Array::from(vec![created_at])),
        ],
    )?;

    table
        .add(RecordBatchIterator::new(vec![Ok(batch)], schema))
        .execute()
        .await?;

    debug!("Annotation added: id={}, path={}", id, path);

    Ok(Annotation {
        id,
        path: path.to_string(),
        note: note.to_string(),
        source: source.to_string(),
        created_at,
    })
}

pub async fn get_annotations(
    db: &Connection,
    container_table: &str,
    path: Option<&str>,
) -> Result<Vec<Annotation>> {
    let table_name = annotations_table_name(container_table);
    let table = match db.open_table(&table_name).execute().await {
        Ok(t) => t,
        Err(_) => return Ok(vec![]),
    };

    let mut query = table.query();
    query = query.select(lancedb::query::Select::Columns(vec![
        "id".to_string(),
        "path".to_string(),
        "note".to_string(),
        "source".to_string(),
        "created_at".to_string(),
    ]));

    if let Some(p) = path {
        let safe_path = p.replace('\'', "''");
        query = query.only_if(format!("path = '{}'", safe_path));
    }

    let results = query.execute().await?.try_collect::<Vec<_>>().await?;

    let mut annotations = Vec::new();
    for batch in results {
        let id_arr = batch.column_by_name("id").and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let path_arr = batch.column_by_name("path").and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let note_arr = batch.column_by_name("note").and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let source_arr = batch.column_by_name("source").and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let ts_arr = batch.column_by_name("created_at").and_then(|c| c.as_any().downcast_ref::<Int64Array>());

        if let (Some(ids), Some(paths), Some(notes), Some(sources), Some(timestamps)) =
            (id_arr, path_arr, note_arr, source_arr, ts_arr)
        {
            for i in 0..batch.num_rows() {
                annotations.push(Annotation {
                    id: ids.value(i).to_string(),
                    path: paths.value(i).to_string(),
                    note: notes.value(i).to_string(),
                    source: sources.value(i).to_string(),
                    created_at: timestamps.value(i),
                });
            }
        }
    }

    annotations.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(annotations)
}

pub async fn delete_annotation(
    db: &Connection,
    container_table: &str,
    annotation_id: &str,
) -> Result<()> {
    let table_name = annotations_table_name(container_table);
    let table = db.open_table(&table_name).execute().await?;

    let safe_id = annotation_id.replace('\'', "''");
    table.delete(&format!("id = '{}'", safe_id)).await?;

    debug!("Annotation deleted: id={}", annotation_id);
    Ok(())
}

pub async fn search_annotations(
    db: &Connection,
    container_table: &str,
    query_vector: &[f32],
    limit: usize,
) -> Result<Vec<(String, String, f32)>> {
    let table_name = annotations_table_name(container_table);
    let table = match db.open_table(&table_name).execute().await {
        Ok(t) => t,
        Err(_) => return Ok(vec![]),
    };

    let row_count = table.count_rows(None).await.unwrap_or(0);
    if row_count == 0 {
        return Ok(vec![]);
    }

    let results = table
        .vector_search(query_vector)?
        .distance_type(lancedb::DistanceType::Cosine)
        .select(lancedb::query::Select::Columns(vec![
            "path".to_string(),
            "note".to_string(),
        ]))
        .limit(limit)
        .execute()
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let mut matches = Vec::new();
    for batch in results {
        let path_arr = batch.column_by_name("path").and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let note_arr = batch.column_by_name("note").and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let dist_arr = batch.column_by_name("_distance").and_then(|c| c.as_any().downcast_ref::<Float32Array>());

        if let (Some(paths), Some(notes), Some(dists)) = (path_arr, note_arr, dist_arr) {
            for i in 0..batch.num_rows() {
                matches.push((
                    paths.value(i).to_string(),
                    format!("[annotation] {}", notes.value(i)),
                    dists.value(i),
                ));
            }
        }
    }

    Ok(matches)
}
