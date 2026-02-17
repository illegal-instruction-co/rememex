# Recall Lite (local-mind)

Windows search sucks. Copilot is creepy. I needed something that finds my sh*t without sending my screen to the cloud.

So I built this.

![SetupContainer](https://github.com/user-attachments/assets/75573638-2fda-4a68-bf61-30aaf5d2ad67)

![Search](https://github.com/user-attachments/assets/4ad0bf74-21a3-4844-8025-45098d9098f3)


Spotlight can't find what you mean. This can.

## What is this?
A **local-first** semantic search engine that lives in your system tray. You type meaning, it finds files.

### Search
- **Hybrid search**: vector similarity + full-text search + cross-encoder reranking
- **Query expansion**: automatically generates keyword variants, strips stop words (EN + TR)
- **Rank fusion (RRF)**: merges vector and FTS results with reciprocal rank fusion
- **Sigmoid scoring**: reranker scores mapped through sigmoid for human-readable percentages
- **Multilingual**: works in English, Turkish, and any language E5-Base supports

### Indexing
- **50+ file formats**: txt, md, pdf, code (rs, py, js, ts, go, java, c, cpp, cs, rb...), config (toml, yaml, json, ini, env), data (csv, sql, log), markup (html, xml, tex, rst, adoc)
- **Dotfiles**: Dockerfile, Makefile, .gitignore, .env, .editorconfig
- **Smart semantic chunking**: splits code at function/class/struct boundaries, markdown at headers, yaml at top-level keys, toml at sections. Chunk sizes tuned per format (code: 1200B, docs: 800B, config: 600B)
- **Incremental**: only re-indexes files that changed (mtime check). Skips unchanged files instantly
- **Streaming batches**: embeds and writes to DB every 64 chunks instead of loading everything into memory

### OCR & Image Intelligence
- **OCR**: extracts text from PNG, JPG, BMP, TIFF, GIF, WebP via Windows built-in OCR engine (zero install, zero config)
- **EXIF metadata**: reads camera make/model, lens, aperture (f/), shutter speed, ISO, focal length, artist, copyright, image description
- **GPS → Location**: reverse geocodes photo coordinates to city/region/country
- **Date intelligence**: parses EXIF timestamps into bilingual human-readable strings — "15 Ocak January 2024, Pazartesi Monday, 14:30 afternoon öğleden sonra, winter kış"
- **All searchable**: you can literally search "photos taken in Istanbul in summer" and get results

### Containers (Sandboxes)
- **Isolated workspaces**: create named containers (Work, Personal, Research) with descriptions
- **Full isolation**: each container gets its own LanceDB table, no data mixing
- **Clean delete**: deleting a container drops its table entirely, zero remnants
- **Default container**: always exists, can't be deleted

### Desktop Integration
- **System tray**: runs in background with Show/Quit menu, click tray icon to toggle
- **Global shortcut**: `Alt + Space` to summon/dismiss from anywhere
- **Mica transparency**: Windows 11 native backdrop material
- **Borderless window**: no titlebar, always on top, skip taskbar — feels like Spotlight
- **Open files**: Enter on a result opens it with default system app

## Tech Stack
- **Frontend**: React + TypeScript (Vite), virtual scrolling, custom modal system
- **Backend**: Rust (Tauri 2), fully async with tokio
- **Vectors**: [LanceDB](https://lancedb.com/) (embedded, no docker, no server)
- **Embedding**: `Multilingual-E5-Base` (768-dim, ~280MB, auto-downloaded)
- **Reranker**: `JINA Reranker v2` (cross-encoder, multilingual, runs on CPU via spawn_blocking)
- **OCR**: Windows.Media.Ocr (built-in, zero install)
- **Geocoding**: `reverse_geocoder` crate (offline, ~10MB dataset, instant lookups)
- **UI**: Windows 11 Fluent / Mica (looks native)

## How to run
You need Rust and Node installed.

```bash
npm install
npm run tauri dev        # dev (downloads models on first run, ~800mb total)
npm run tauri build      # release (use this for real performance)
```

## Usage
| Shortcut | Action |
|---|---|
| `Alt + Space` | Toggle search window |
| `Ctrl + O` | Index a new folder |
| `↑ ↓` | Navigate results |
| `Enter` | Open selected file |
| `Esc` | Clear search |
| `Shift + Delete` | Clear current container's index |

## Configuration
`%AppData%\com.recall-lite.app\config.json`

```json
{
  "embedding_model": "MultilingualE5Base",
  "containers": {
    "Default": { "description": "", "indexed_paths": [] }
  },
  "active_container": "Default"
}
```

**Supported models**: AllMiniLML6V2, MultilingualE5Small, MultilingualE5Base

Auto-migrates old config formats. Model loads with 3 retry attempts. Legacy `.fastembed_cache` cleaned up automatically.

## Performance
- Tested on 10k+ files
- Indexing streams in 64-chunk batches (constant memory)
- Search: hybrid vector + FTS + reranker, all parallelized
- Release builds only — debug mode is 10x slower, that's normal

## Logs
`%AppData%\com.recall-lite.app\recall.log`

## License
MIT. Do whatever.
