pub mod chunking;
pub mod db;
pub mod embedding;
pub mod file_io;
pub mod ocr;
pub mod search;

use anyhow::Result;
use arrow_array::RecordBatchIterator;
use fastembed::TextEmbedding;
use lancedb::connection::Connection;

use walkdir::WalkDir;

pub use chunking::expand_query;
pub use db::reset_index;
pub use embedding::{embed_query, load_model, load_reranker, rerank_results};
pub use search::{hybrid_merge, search_files, search_fts};

const ANN_INDEX_THRESHOLD: usize = 256;
const EMBED_BATCH_SIZE: usize = 64;

pub async fn index_directory<F>(
    root_dir: &str,
    table_name: &str,
    db: &Connection,
    model: &mut TextEmbedding,
    progress_callback: F,
) -> Result<usize>
where
    F: Fn(usize, usize, String) + Send + 'static,
{
    let dim = embedding::get_model_dimension(model)?;
    let table = db::get_or_create_table(db, table_name, dim).await?;

    let existing_mtimes = db::get_indexed_mtimes(&table).await.unwrap_or_default();

    let all_files: Vec<_> = WalkDir::new(root_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect();
    let total_files = all_files.len();

    let mut pending_chunks: Vec<db::PendingChunk> = Vec::new();
    let mut files_seen = 0;
    let mut current_file = 0;

    for path in &all_files {
        current_file += 1;
        let path_str = path.to_string_lossy().to_string();
        let mtime = file_io::get_file_mtime(path);

        if let Some(&existing_mtime) = existing_mtimes.get(&path_str) {
            if existing_mtime == mtime {
                files_seen += 1;
                progress_callback(current_file, total_files, path_str);
                continue;
            }
        }

        let text = match file_io::read_file_content(path) {
            Some(t) if !t.trim().is_empty() => t,
            _ => {
                progress_callback(current_file, total_files, path_str);
                continue;
            }
        };

        let safe_path = path_str.replace('\'', "''");
        let _ = table.delete(&format!("path = '{}'", safe_path)).await;

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let chunks = chunking::semantic_chunk(&text, &ext);
        for chunk in chunks {
            pending_chunks.push(db::PendingChunk {
                path: path_str.clone(),
                content: chunk,
                mtime,
            });
        }

        progress_callback(current_file, total_files, path_str);
        files_seen += 1;
    }

    if pending_chunks.is_empty() {
        progress_callback(total_files, total_files, "Done -- no new files".to_string());
        return Ok(0);
    }

    let file_set: std::collections::HashSet<&str> = pending_chunks.iter().map(|c| c.path.as_str()).collect();
    let files_indexed = file_set.len();

    let total_batches = (pending_chunks.len() + EMBED_BATCH_SIZE - 1) / EMBED_BATCH_SIZE;

    for (batch_idx, batch_start) in (0..pending_chunks.len()).step_by(EMBED_BATCH_SIZE).enumerate() {
        let batch_end = (batch_start + EMBED_BATCH_SIZE).min(pending_chunks.len());
        let batch_chunks = &pending_chunks[batch_start..batch_end];

        progress_callback(
            total_files,
            total_files,
            format!("Embedding batch {}/{}", batch_idx + 1, total_batches),
        );

        let texts: Vec<String> = batch_chunks.iter().map(|c| c.content.clone()).collect();
        let embeddings = embedding::embed_passages(model, texts)?;

        let records: Vec<db::Record> = batch_chunks
            .iter()
            .zip(embeddings)
            .map(|(chunk, vector)| db::Record {
                path: chunk.path.clone(),
                content: chunk.content.clone(),
                vector,
                mtime: chunk.mtime,
            })
            .collect();

        let batch = db::create_record_batch(records)?;
        let schema = batch.schema();
        table
            .add(RecordBatchIterator::new(vec![Ok(batch)], schema))
            .execute()
            .await?;
    }

    if files_seen >= ANN_INDEX_THRESHOLD {
        progress_callback(total_files, total_files, "Building vector index...".to_string());
        let _ = db::build_ann_index(&table).await;
    }

    progress_callback(total_files, total_files, "Building search index...".to_string());
    let _ = db::build_fts_index(&table).await;

    Ok(files_indexed)
}
