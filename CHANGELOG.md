# Changelog

## [0.9.0] - 2026-05-14

### Fixed / Improved

- **搜索质量提升**: ripgrep 搜索改用关键词交替匹配（`word1|word2|word3`）替代短语搜索
  - `ask`、`patch`、`run`、`search` 命令均已更新
  - 显著提升相关代码检索的召回率
- **索引搜索优化**: 按单个关键词分别查询符号索引，合并去重结果
  - 替代原来的整体模式匹配，大幅提升符号检索精度
- **请求重试机制**: ModelClient 新增指数退避重试（3 次尝试，2/4/8 秒间隔）
  - 支持 429 (Rate Limit) 和 5xx (Server Error) 自动重试
  - 连接错误同样触发重试
- **上下文大小可配置**: `config.toml` 新增 `context_limit` 参数
  - 控制发送给模型的上下文字符数上限（默认 8000）
  - 小模型可设为 4000 以避免截断
- **Review Prompt 增强**: 审查提示词要求分析变更文件整体代码，而非仅 diff 行
  - 增加安全检查指令（弱密码哈希、缺少输入验证、注入风险等）
- **search 命令增强**: 现在同时搜索 ripgrep 结果和索引符号

### Added

- **`test/scripts/`**: 完整的评估测试框架
  - `setup_project.py` — 创建测试项目并配置 dumbcoder
  - `evaluate.py` — 自动化测试运行 + LLM 评估
  - 所有 LLM 配置通过环境变量传入

## [0.8.0] - 2026-05-08

### Added

- **插件系统**: 自定义命令和 prompt 模板
  - `.dumbcoder/plugins/*.toml` — 每个文件定义一个自定义命令
  - `dumbcoder run <name> <query>` — 执行插件命令
  - 插件定义 `name`、`description`、`system_prompt` 字段
  - 执行流程：关键词提取 → ripgrep 搜索 + 索引 → 上下文组装 → 调用模型
- **Prompt 模板覆盖**: `config.toml` 中的 `[prompts]` 节
  - 可覆盖内置命令的 system prompt（ask、explain、review、test、patch）
  - 示例：`[prompts]\nask = "你的自定义 prompt..."`
  - 未配置时使用内置默认 prompt

### Documentation

- 更新 `docs/commands.zh.md` — 新增 dumbcoder run 和插件系统说明
- 更新 `docs/commands.en.md` — Added dumbcoder run and plugin system docs

## [0.7.0] - 2026-05-08

### Added

- **扩展语言索引支持**: 新增 5 种语言的 tree-sitter AST 解析
  - C (`.c`, `.h`) — 提取函数、结构体、枚举、类型定义、include
  - C++ (`.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx`) — 提取函数、类、结构体、枚举、命名空间、include
  - JavaScript (`.js`, `.jsx`, `.mjs`) — 提取函数、方法、类、常量声明、import
  - Ruby (`.rb`, `.rake`) — 提取方法、单例方法、类、模块
  - Kotlin (`.kt`, `.kts`) — 提取函数、类、对象声明、属性、import

### Dependencies Added

- `tree-sitter-c` 0.24 — C 语法
- `tree-sitter-cpp` 0.23 — C++ 语法
- `tree-sitter-javascript` 0.25 — JavaScript 语法
- `tree-sitter-ruby` 0.23 — Ruby 语法
- `tree-sitter-kotlin-ng` 1.1 — Kotlin 语法

### Documentation

- 更新 `docs/commands.zh.md` — 新增支持的语言列表
- 更新 `docs/commands.en.md` — Updated supported languages list

## [0.6.0] - 2026-05-08

### Added

- **多模型支持**: 支持 Ollama、OpenAI API、OpenAI 兼容 API（vLLM、DeepSeek 等）
  - `ModelClient` 使用 `Provider` 枚举分派到不同后端
  - Ollama: 使用 `/api/chat` + `/api/generate` 回退（原有行为）
  - OpenAI: 使用 `/v1/chat/completions`，支持 `Authorization: Bearer` 认证
  - `openai_compatible`: 与 OpenAI 相同协议，适用于 vLLM、DeepSeek 等
- **`ModelConfig` 新增字段**:
  - `api_key: Option<String>` — API 密钥（OpenAI 必需，openai_compatible 可选）
  - `timeout_seconds: Option<u64>` — 可配置超时时间（默认 120 秒）
- **`ModelConfig::validate()`**: 启动时验证 provider 和配置一致性
- 移除未使用的 `chat()` 方法（保留 `ChatMessage` 类型供 TUI 使用）

### Multi-provider Support

- Supports Ollama, OpenAI API, and OpenAI-compatible APIs (vLLM, DeepSeek, etc.)
- New config fields: `api_key`, `timeout_seconds`
- Provider validation on startup

## [0.5.0] - 2026-05-08

### Added

- **`dumbcoder patch`**: 受控代码修改补丁生成
  - 根据自然语言描述搜索相关代码、生成 unified diff
  - `git apply --check` 验证补丁可应用性
  - 用户确认后才实际应用补丁
  - 应用后自动运行测试（自动检测测试命令）
  - 测试失败时自动回滚（`git apply --reverse`）
  - 所有操作写入 `.dumbcoder/logs/` 审计日志（JSON 格式）
- **`src/audit.rs`**: 审计日志模块
  - `AuditEntry` 结构体：记录时间戳、命令、描述、读取文件、生成的 diff、应用结果、测试结果、错误信息
  - `log_entry()` 函数：将审计条目序列化为 JSON 写入 `.dumbcoder/logs/`

### Documentation

- 更新 `docs/commands.zh.md` — 新增 dumbcoder patch 命令说明
- 更新 `docs/commands.en.md` — Added dumbcoder patch command docs

## [0.4.0] - 2026-05-08

### Added

- **`dumbcoder test`**: 单元测试生成
  - 为指定文件或函数生成全面的单元测试
  - 支持 `--symbol` 参数精确定位目标函数
  - 自动检测项目测试框架（cargo test、pytest、go test、npm test、mvn test、gradle test）
  - 查找已有测试文件作为风格参考
  - 使用索引精确定位符号代码范围
  - 覆盖正常、边界和错误情况
- **`dumbcoder review`**: Git diff 审查
  - 审查未暂存修改（默认）、已暂存修改（`--staged`）、指定范围（`--diff`）
  - 解析 diff 文件列表
  - 结合索引符号信息提供上下文
  - 输出结构化审查报告（风险等级、问题列表、改进建议）
- **`src/git.rs`**: Git 工具模块
  - `get_staged_diff` / `get_diff_range` / `get_unstaged_diff`
  - `detect_test_command` — 自动检测项目测试命令
  - `parse_changed_files` — 从 diff 中提取变更文件列表

### Documentation

- 更新 `docs/commands.zh.md` — 新增 dumbcoder test/review 命令说明
- 更新 `docs/commands.en.md` — Added dumbcoder test/review command docs

## [0.3.0] - 2026-05-08

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
