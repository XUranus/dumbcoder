# Development Plan

## Phase 1: Rust CLI MVP (Current)

**Duration**: 2-4 weeks

**Goal**: Minimum viable CLI tool.

**Implemented**:
- `dumbcoder init` — Initialize project configuration
- `dumbcoder ask "question"` — Codebase Q&A
- `dumbcoder explain <file>` — Explain files/functions
- `dumbcoder search "query"` — Smart code search
- Local Ollama model integration
- Basic security filtering (file blacklist, path sandbox)
- TOML configuration support

**Acceptance Criteria**:
- [x] Runs on development machines
- [x] Connects to local Ollama model service
- [x] Can answer questions about real repositories
- [x] Can explain files and functions
- [x] Answers reference relevant file paths

## Phase 2: Code Indexing & AST

**Duration**: 4-6 weeks

**Goal**: Improve code location and context accuracy.

**Planned Features**:
- `dumbcoder index` — Build code index
- File indexing
- Symbol indexing
- tree-sitter function-level parsing
- AST context extraction
- Git changed files incremental indexing
- Local index database (sqlite-vec)

## Phase 3: TUI Interface

**Duration**: 3-5 weeks

**Goal**: Enhanced terminal interaction experience.

**Planned Features**:
- `dumbcoder tui` — Interactive TUI mode
- Chat panel
- File reference panel
- Code snippet panel
- Diff preview panel
- Test results panel
- Keyboard shortcuts
- History browsing

## Phase 4: Test Generation & Review

**Duration**: 4-6 weeks

**Goal**: Integrate into real development workflows.

**Planned Features**:
- `dumbcoder test` — Unit test generation
- `dumbcoder review` — Git diff review
- Test command detection
- Test execution and log summarization
- Review report generation

## Phase 5: Controlled Patch Agent

**Duration**: 6-8 weeks

**Goal**: Support small-scale code modifications.

**Planned Features**:
- `dumbcoder patch` — Controlled code modification
- Modification plan generation
- Diff generation
- `git apply --check`
- User confirmation before apply
- Automatic test execution
- Failure rollback
- Audit logging
