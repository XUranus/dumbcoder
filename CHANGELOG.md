# Changelog

## [0.3.0] - 2026-05-08

### Added

- **`dumbcoder tui`**: 交互式 TUI 模式
  - 多轮对话界面，左侧面板显示聊天记录
  - 上下文面板，右侧面板显示引用文件和索引符号
  - 输入栏，支持光标移动、编辑、发送
  - 实时状态指示（Ready / Thinking / Error）
  - 完整键盘快捷键支持
- **TUI 模块架构**:
  - `src/tui/mod.rs` — TUI 入口，终端初始化/恢复，事件循环
  - `src/tui/app.rs` — 应用状态管理（消息、输入、滚动、面板切换）
  - `src/tui/event.rs` — crossterm 事件处理
  - `src/tui/ui.rs` — ratatui 渲染（标题栏、聊天面板、上下文面板、输入栏）
  - `src/tui/action.rs` — 业务逻辑（ask/search/explain 执行）
  - `src/cmd/tui.rs` — CLI 入口委托

### Dependencies Added

- `ratatui` 0.29 — 终端 UI 框架
- `crossterm` 0.28 — 终端事件处理

### Documentation

- 新增 `docs/tui.zh.md` / `docs/tui.en.md` — TUI 使用指南（中英双语）
- 更新 `docs/commands.zh.md` / `docs/commands.en.md` — 新增 dumbcoder tui 命令说明

## [0.2.0] - 2026-05-08

### Added

- **`dumbcoder index`**: 代码索引命令，支持 `--full`（全量）和 `--changed`（增量）模式
- **tree-sitter AST 解析**: 支持 Rust、Go、Python、TypeScript、Java 五种语言的 AST 解析
- **符号提取**: 从 AST 中提取函数、结构体、类、枚举、trait、impl、import 等符号
- **SQLite 索引存储**: 使用 rusqlite (bundled) 将符号存储到 `.dumbcoder/index/symbols.db`
- **增量索引**: 通过 git diff 检测变更文件，仅重新索引变更的文件
- **智能上下文增强**: `dumbcoder ask` 和 `dumbcoder explain` 自动使用索引结果提供精确的符号级上下文
- **`dumbcoder ask` 增强**: 合并 ripgrep 搜索结果 + 索引符号结果作为上下文
- **`dumbcoder explain --symbol` 增强**: 使用索引精确定位符号行范围，替代原有的 brace-depth 启发式方法

### Dependencies Added

- `tree-sitter` 0.24 — 核心解析器
- `tree-sitter-rust` 0.23 — Rust 语法
- `tree-sitter-go` 0.23 — Go 语法
- `tree-sitter-python` 0.23 — Python 语法
- `tree-sitter-typescript` 0.23 — TypeScript 语法
- `tree-sitter-java` 0.23 — Java 语法
- `rusqlite` 0.32 (bundled) — SQLite 数据库

### Documentation

- 更新 `docs/commands.zh.md` — 新增 `dumbcoder index` 命令说明
- 更新 `docs/commands.en.md` — Added `dumbcoder index` command documentation

## [0.1.0] - 2026-05-08

### Added

- **CLI 框架**: 基于 clap 的命令行界面，支持 8 个子命令 (`init`, `ask`, `explain`, `search`, `test`, `review`, `patch`, `tui`)
- **`dumbcoder init`**: 初始化项目配置，自动检测项目语言和 Git 仓库
- **`dumbcoder ask`**: 代码库问答 — ripgrep 搜索相关代码 → 安全过滤 → 上下文组装 → 调用本地模型
- **`dumbcoder explain`**: 解释文件或指定函数/符号，支持 `--symbol` 参数提取代码块
- **``dumbcoder search`**: 智能代码搜索，调用 ripgrep，支持 `--lang` 语言过滤
- **模型客户端**: Ollama API 客户端，支持 `/api/chat` 和 `/api/generate` 两种模式
- **配置管理**: TOML 配置文件 (`.dumbcoder/config.toml`)，包含 model、index、security、commands 四个模块
- **安全层**: 文件黑名单、路径沙箱、命令白名单、敏感文件检测
- **上下文组装**: 基于搜索结果构建模型提示词上下文，支持大小限制

### Documentation

- `docs/architecture.zh.md` / `docs/architecture.en.md` — 系统架构文档（中英双语）
- `docs/development-plan.zh.md` / `docs/development-plan.en.md` — 开发计划文档（中英双语）
- `docs/commands.zh.md` / `docs/commands.en.md` — 命令参考文档（中英双语）
- `docs/security.zh.md` / `docs/security.en.md` — 安全设计文档（中英双语）

### Tech Stack

- Rust (edition 2021)
- clap 4 (CLI)
- tokio (async runtime)
- reqwest (HTTP client)
- serde + toml (config)
- ripgrep (code search)
- colored (terminal output)

### Stubs (Coming Soon)

- `dumbcoder test` — 单元测试生成
- `dumbcoder review` — Git diff 审查
- `dumbcoder patch` — 受控代码修改
- `dumbcoder tui` — 交互式 TUI
