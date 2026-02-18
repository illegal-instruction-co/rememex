# MCP server

you know how every AI editor wants you to install some extension or connect to some API? forget that. recall-lite has an MCP server built in. one exe, stdin/stdout, done.

plug it into cursor, claude desktop, copilot, whatever. your AI can now search your local files without you copy-pasting paths like an animal.

## tools

- **`recall_search(query, container?)`** -- full pipeline. vector search → keyword search → hybrid merge → JINA reranker. same quality as the GUI. returns paths, snippets, scores
- **`recall_list_containers()`** -- dumps your containers. names, paths, which one's active

## get the binary

grab `recall-mcp.exe` from [releases](https://github.com/illegal-instruction-co/recall-lite/releases).

or build it yourself if you're into that:
```bash
cargo build --bin recall-mcp --release
# sits in src-tauri/target/release/
```

## before you start

index some folders in the main app first. the MCP server doesn't index anything, it just searches. no index = no results = you'll think it's broken.

## hook it up

### cursor

settings → MCP → add server. or just edit `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "recall-lite": {
      "command": "C:\\Users\\YOU\\recall-mcp.exe"
    }
  }
}
```

restart. done.

### claude desktop

edit `%AppData%\Claude\claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "recall-lite": {
      "command": "C:\\Users\\YOU\\recall-mcp.exe"
    }
  }
}
```

restart. done.

### VS code copilot

`.vscode/mcp.json` or user settings:

```json
{
  "mcp": {
    "servers": {
      "recall-lite": {
        "command": "C:\\Users\\YOU\\recall-mcp.exe"
      }
    }
  }
}
```

### anything else

stdio transport. point at the exe. no args, no env vars, no ports, no docker. just the path.

## what happens under the hood

on launch it:
1. opens the same LanceDB the main app uses (`%AppData%\com.recall-lite.app\lancedb`)
2. loads embedding model + reranker from local cache (`%AppData%\com.recall-lite.app\models`)
3. reads your config (`%AppData%\com.recall-lite.app\config.json`)
4. sits on stdin waiting for queries

first launch is slow (~3-5 sec) because it loads ~1.1GB of embedding model weights + ~1GB reranker. after that it's instant.

## stuff that might confuse you

**no results** -- you didn't index anything. open the main app, index a folder, try again

**slow first query** -- model loading. chill. next ones are fast

**server not showing up** -- check the exe path. absolute path. double backslashes on windows. yes it's annoying

**searching wrong stuff** -- defaults to active container. pass `container: "Whatever"` to pick a different one

## privacy

everything local. reads local DB, uses local models, talks over stdio not network. your files stay on your machine. that's the whole point.
