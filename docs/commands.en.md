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

## dumbcoder index

Build or update code index. Uses tree-sitter to parse ASTs, extract symbols (functions, structs, classes, etc.), and store them in a local SQLite database.

```bash
dumbcoder index            # Incremental index (changed files only)
dumbcoder index --full     # Full re-index of all files
dumbcoder index --changed  # Incremental index (same as default)
```

**Features**:
- Scans project source files (Rust, Go, Python, TypeScript, Java)
- Parses ASTs with tree-sitter
- Extracts functions, structs, classes, enums, traits, impls, imports
- Stores in `.dumbcoder/index/symbols.db` (SQLite)
- Supports incremental indexing (detects changed files via git diff)
- Supports full re-indexing

**Index Database**:
- Auto-creates `.dumbcoder/index/symbols.db`
- Contains `files` and `symbols` tables
- Supports fuzzy symbol name search
- `ask` and `explain` commands automatically use index results

**Supported Languages**:
- Rust (`.rs`)
- Go (`.go`)
- Python (`.py`)
- TypeScript/TSX (`.ts`, `.tsx`)
- Java (`.java`)
- C (`.c`, `.h`)
- C++ (`.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx`)
- JavaScript/JSX (`.js`, `.jsx`, `.mjs`)
- Ruby (`.rb`, `.rake`)
- Kotlin (`.kt`, `.kts`)

## Configuration

Configuration file at `.dumbcoder/config.toml`:

```toml
[model]
provider = "ollama"          # ollama | openai | openai_compatible
base_url = "http://127.0.0.1:11434"
model = "qwen2.5-coder:7b"
# api_key = "sk-..."        # Required for openai, optional for openai_compatible
# timeout_seconds = 120     # Request timeout (default: 120)

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

**Supported Providers**:

| Provider | Description | Requires `api_key` |
|----------|-------------|-------------------|
| `ollama` | Local Ollama server (default) | No |
| `openai` | OpenAI API (GPT-4o, etc.) | Yes |
| `openai_compatible` | vLLM, DeepSeek, or any OpenAI-compatible API | Optional |

**Example configs**:

```toml
# OpenAI
[model]
provider = "openai"
base_url = "https://api.openai.com"
model = "gpt-4o"
api_key = "sk-..."
timeout_seconds = 180

# vLLM
[model]
provider = "openai_compatible"
base_url = "http://10.0.0.5:8000"
model = "Qwen/Qwen2.5-Coder-32B-Instruct"
timeout_seconds = 300
```

## dumbcoder tui

Enter interactive TUI mode.

```bash
dumbcoder tui
```

**Features**:
- Multi-turn conversation: interactive chat with AI in the terminal
- Code context: view referenced files and symbols
- Keyboard navigation: full keyboard shortcut support

**Key Bindings**:
- `Enter` — Send message
- `Tab` — Switch panel (chat / context)
- `Esc` — Switch panel or quit
- `Up/Down` — Scroll context panel
- `PgUp/PgDn` — Page scroll
- `Ctrl+C` — Quit
- `Ctrl+L` — Clear chat

See [TUI Guide](tui.en.md) for details.

## dumbcoder test

Generate unit tests for a file or function.

```bash
dumbcoder test src/order/service.rs
dumbcoder test src/order/service.rs --symbol cancel_order
```

**Parameters**:
- `--symbol` — Specify the function or class to generate tests for

**Features**:
- Analyzes the target function's inputs, outputs, and logic
- Auto-detects project test framework (cargo test, pytest, go test, npm test, etc.)
- Finds existing test files as style references
- Generates tests covering normal, edge, and error cases
- Outputs ready-to-use test code

## dumbcoder review

Review git diff and provide a structured code review report.

```bash
dumbcoder review              # Review unstaged changes
dumbcoder review --staged     # Review staged changes
dumbcoder review --diff main...HEAD  # Review branch diff
```

**Parameters**:
- `--staged` — Review staged changes (git diff --cached)
- `--diff` — Specify diff range

**Features**:
- Reads git diff
- Analyzes changed files and code
- Combines symbol context from the index
- Outputs a structured review report including:
  - Risk level per file (Low/Medium/High)
  - Issues found (potential bugs, edge cases, logic errors)
  - Suggestions (test coverage, security concerns)

## dumbcoder patch

Generate a controlled code patch from a natural language description.

```bash
dumbcoder patch "fix error handling in order cancellation"
dumbcoder patch "add input validation to user registration"
```

**Flow**:
1. Extract keywords from description
2. Search for relevant code (ripgrep + index)
3. Build code context
4. Send to model to generate a unified diff
5. Validate diff with `git apply --check`
6. Display the generated patch
7. Prompt for user confirmation
8. Apply the patch with `git apply`
9. Run tests (auto-detected)
10. Rollback if tests fail
11. Log to `.dumbcoder/logs/` as an audit entry

**Safety**:
- Diff is validated before applying (`git apply --check`)
- User must confirm before the patch is applied
- Tests are automatically run after applying
- Patch is rolled back if tests fail
- All actions are audit-logged
