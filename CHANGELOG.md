# Changelog

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
