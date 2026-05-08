# TUI Guide

## Launch

```bash
dumbcoder tui
```

Must be run in a project directory containing `.dumbcoder/` or `.git/`.

## Layout

```
┌─────────────────────────────────────────────────────┐
│ dumbcoder │ Ready │ 2 messages │ Ctrl+C: quit...    │
├──────────────────┼──────────────────────────────────┤
│                  │                                  │
│  Chat            │  Context                         │
│                  │                                  │
│  > your question │  ── Symbols ──                   │
│  < AI response   │  function cancel_order           │
│                  │    src/order/service.rs:42       │
│                  │                                  │
│                  │  ── Referenced Files ──          │
│                  │  ── src/order/service.rs ──      │
│                  │  pub fn cancel_order(...)        │
├──────────────────┴──────────────────────────────────┤
│  > type question...                        [Enter]  │
└─────────────────────────────────────────────────────┘
```

## Usage

1. Type your question in the bottom input bar
2. Press `Enter` to send
3. The AI response appears in the left panel
4. Referenced files and symbols appear in the right panel

## Key Bindings

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Tab` | Switch focus panel (chat / context) |
| `Esc` | Switch panel, or quit if input is empty |
| `Up` / `Down` | Scroll context panel |
| `PgUp` / `PgDn` | Page scroll in context panel |
| `Ctrl+C` | Quit TUI |
| `Ctrl+L` | Clear chat history |
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character after cursor |
| `Left` / `Right` | Move cursor |
| `Home` / `End` | Jump to start/end of line |

## Status Indicators

- **Ready** (green) — Ready for new input
- **Thinking...** (yellow + spinner) — Waiting for model response
- **Error** (red) — An error occurred (details shown in chat panel)

## Use Cases

- **Code Q&A**: Ask about project structure, function responsibilities, call relationships
- **Code Explanation**: View explanations of specific files or functions via the `explain` command
- **Code Search**: Search for relevant code via the `search` command
- **Multi-turn Conversation**: Supports continuous questioning with context

## Prerequisites

1. Run `dumbcoder init` to initialize the project first
2. Local Ollama service must be running (default `http://127.0.0.1:11434`)
3. Recommend running `dumbcoder index` first for better results
