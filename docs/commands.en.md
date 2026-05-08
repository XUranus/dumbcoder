# Command Reference

## dumbcoder init

Initialize project configuration.

```bash
dumbcoder init
```

**Features**:
- Creates `.dumbcoder/` directory
- Generates default `config.toml`
- Detects project language (Rust, Go, Python, Node.js, Java, etc.)
- Detects Git repository

## dumbcoder ask

Codebase Q&A.

```bash
dumbcoder ask "Where is the order cancellation logic?"
dumbcoder ask "How does authentication work?"
```

**Flow**:
1. Extract keywords from question
2. Search relevant code with ripgrep
3. Security filtering (exclude sensitive files)
4. Assemble code context
5. Send to local model
6. Output formatted answer with file references

**Configuration**:
- `model.base_url` — Model service URL
- `model.model` — Model name
- `index.ignore` — Directories to ignore

## dumbcoder explain

Explain files or functions.

```bash
dumbcoder explain src/order/service.rs
dumbcoder explain src/order/service.rs --symbol cancel_order
```

**Features**:
- Reads specified file
- Optionally extracts code for a specific symbol
- Sends to model for explanation
- Outputs structured explanation

## dumbcoder search

Smart code search.

```bash
dumbcoder search "order status flow"
dumbcoder search "cancel order" --lang rust
```

**Parameters**:
- `--lang` — Filter by programming language

**Features**:
- Calls ripgrep for search
- Security filtering
- Outputs file paths and line numbers

## Configuration

Configuration file at `.dumbcoder/config.toml`:

```toml
[model]
provider = "ollama"
base_url = "http://127.0.0.1:11434"
model = "qwen2.5-coder:7b"

[index]
enabled = true
db_path = ".dumbcoder/index"
ignore = [".git", "target", "node_modules", "dist"]

[security]
allow_write = false
allow_network = false
max_command_seconds = 60
max_output_bytes = 20000

[commands]
allow = ["rg", "git status", "git diff", "git log", "git show"]
```

## Coming Soon

- `dumbcoder test` — Generate unit tests
- `dumbcoder review` — Review Git diffs
- `dumbcoder patch` — Generate controlled code patches
- `dumbcoder tui` — Interactive TUI mode
