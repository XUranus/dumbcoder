# dumbcoder System Architecture

## Overview

dumbcoder uses a layered architecture with five tiers:

```
┌──────────────────────────────────────────────┐
│              Rust CLI / TUI                  │
│  ask / explain / search / test / review / patch│
└──────────────────────────────────────────────┘
                     │
                     v
┌──────────────────────────────────────────────┐
│           Agent Orchestrator                 │
│  Task splitting / Context / Tool calls / Policy│
└──────────────────────────────────────────────┘
                     │
                     v
┌──────────────────────────────────────────────┐
│              Tool Layer                      │
│ rg / git / tree-sitter / LSP / test / diff    │
└──────────────────────────────────────────────┘
                     │
                     v
┌──────────────────────────────────────────────┐
│           Code Index & RAG Layer             │
│ BM25 / Vector DB / AST Index / Git History    │
└──────────────────────────────────────────────┘
                     │
                     v
┌──────────────────────────────────────────────┐
│           Local Model Service                │
│ Ollama / llama.cpp / vLLM / SGLang            │
│ Qwen2.5-Coder-7B / 14B / other local models   │
└──────────────────────────────────────────────┘
                     │
                     v
┌──────────────────────────────────────────────┐
│           Security & Audit Layer             │
│ File permissions / Command whitelist / Audit log│
└──────────────────────────────────────────────┘
```

## Core Modules

| Module | File | Responsibility |
|--------|------|----------------|
| CLI Entry | `main.rs` | Command parsing, subcommand dispatch |
| Config | `config.rs` | Load/save TOML configuration |
| Model Client | `model.rs` | Call Ollama API |
| Code Context | `context.rs` | Assemble code context, format prompts |
| Code Search | `cmd/search.rs` | Ripgrep-based search |
| Code Q&A | `cmd/ask.rs` | Search + context + model Q&A |
| Code Explain | `cmd/explain.rs` | File/function explanation |
| Security | `security.rs` | File blacklist, path sandbox, command whitelist |
| Utilities | `util.rs` | Terminal output formatting, project detection |

## Data Flow

User question → ripgrep search → security filter → context assembly → local model call → formatted output

## Tech Stack

- **Language**: Rust
- **CLI Framework**: clap (derive)
- **Async Runtime**: tokio
- **HTTP Client**: reqwest
- **Config**: serde + toml
- **Model Service**: Ollama (local)
- **Code Search**: ripgrep
