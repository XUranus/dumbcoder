# 命令参考

## dumbcoder init

初始化项目配置。

```bash
dumbcoder init
```

**功能**:
- 创建 `.dumbcoder/` 目录
- 生成默认 `config.toml`
- 检测项目语言（Rust、Go、Python、Node.js、Java 等）
- 检测是否为 Git 仓库

## dumbcoder ask

代码库问答。

```bash
dumbcoder ask "订单取消逻辑在哪里？"
dumbcoder ask "How does authentication work?"
```

**流程**:
1. 从问题中提取关键词
2. 使用 ripgrep 搜索相关代码
3. 安全过滤（排除敏感文件）
4. 组装代码上下文
5. 发送到本地模型
6. 输出格式化回答和引用文件

**配置**:
- `model.base_url` — 模型服务地址
- `model.model` — 模型名称
- `index.ignore` — 忽略目录

## dumbcoder explain

解释文件或函数。

```bash
dumbcoder explain src/order/service.rs
dumbcoder explain src/order/service.rs --symbol cancel_order
```

**功能**:
- 读取指定文件
- 可选提取指定 symbol 的代码
- 发送到模型生成解释
- 输出结构化解释

## dumbcoder search

智能代码搜索。

```bash
dumbcoder search "订单状态流转"
dumbcoder search "cancel order" --lang rust
```

**参数**:
- `--lang` — 按编程语言过滤

**功能**:
- 调用 ripgrep 搜索
- 安全过滤
- 输出文件路径和行号

## dumbcoder index

构建或更新代码索引。使用 tree-sitter 解析 AST，提取函数、结构体、类等符号，存储到本地 SQLite 数据库。

```bash
dumbcoder index            # 增量索引（仅变更文件）
dumbcoder index --full     # 全量索引
dumbcoder index --changed  # 增量索引（同默认）
```

**功能**:
- 扫描项目源文件（Rust、Go、Python、TypeScript、Java）
- 使用 tree-sitter 进行 AST 解析
- 提取函数、结构体、类、枚举、trait、impl、import 等符号
- 存储到 `.dumbcoder/index/symbols.db`（SQLite）
- 支持增量索引（通过 git diff 检测变更文件）
- 支持全量重新索引

**索引数据库**:
- 自动创建 `.dumbcoder/index/symbols.db`
- 包含 `files` 表和 `symbols` 表
- 支持按名称模糊搜索符号
- `ask` 和 `explain` 命令自动使用索引结果

**支持的语言**:
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

## 配置文件

配置文件位于 `.dumbcoder/config.toml`：

```toml
[model]
provider = "ollama"          # ollama | openai | openai_compatible
base_url = "http://127.0.0.1:11434"
model = "qwen2.5-coder:7b"
# api_key = "sk-..."        # openai 必需，openai_compatible 可选
# timeout_seconds = 120     # 请求超时（默认 120 秒）

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

**支持的模型提供方**:

| 提供方 | 说明 | 需要 `api_key` |
|--------|------|----------------|
| `ollama` | 本地 Ollama 服务（默认） | 否 |
| `openai` | OpenAI API（GPT-4o 等） | 是 |
| `openai_compatible` | vLLM、DeepSeek 或任何 OpenAI 兼容 API | 可选 |

**配置示例**:

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

进入交互式 TUI 模式。

```bash
dumbcoder tui
```

**功能**:
- 多轮对话：在终端中与 AI 进行交互式对话
- 代码上下文：查看 AI 引用的文件和符号
- 键盘操作：完整键盘快捷键支持

**快捷键**:
- `Enter` — 发送消息
- `Tab` — 切换面板（聊天 / 上下文）
- `Esc` — 切换面板或退出
- `Up/Down` — 滚动上下文面板
- `PgUp/PgDn` — 翻页滚动
- `Ctrl+C` — 退出
- `Ctrl+L` — 清空对话

详见 [TUI 使用指南](tui.zh.md)。

## dumbcoder test

为指定文件或函数生成单元测试。

```bash
dumbcoder test src/order/service.rs
dumbcoder test src/order/service.rs --symbol cancel_order
```

**参数**:
- `--symbol` — 指定要生成测试的函数或类名

**功能**:
- 分析目标函数的输入输出和逻辑
- 自动检测项目测试框架（cargo test、pytest、go test、npm test 等）
- 查找已有测试文件作为风格参考
- 生成覆盖正常、边界和错误情况的测试用例
- 输出可直接使用的测试代码

## dumbcoder review

审查 Git diff，提供结构化代码审查报告。

```bash
dumbcoder review              # 审查未暂存的修改
dumbcoder review --staged     # 审查已暂存的修改
dumbcoder review --diff main...HEAD  # 审查分支差异
```

**参数**:
- `--staged` — 审查已暂存的修改（git diff --cached）
- `--diff` — 指定 diff 范围

**功能**:
- 读取 git diff
- 分析变更的文件和代码
- 结合索引中的符号信息提供上下文
- 输出结构化审查报告，包含：
  - 每个文件的风险等级（低/中/高）
  - 发现的问题（潜在 bug、边界情况、逻辑错误）
  - 改进建议（测试覆盖、安全考量）

## dumbcoder patch

根据自然语言描述生成受控代码修改补丁。

```bash
dumbcoder patch "修复订单取消中的错误处理"
dumbcoder patch "为用户注册添加输入验证"
```

**流程**:
1. 从描述中提取关键词
2. 搜索相关代码（ripgrep + 索引）
3. 组装代码上下文
4. 发送到模型生成 unified diff
5. 用 `git apply --check` 验证补丁
6. 展示生成的补丁
7. 提示用户确认
8. 用 `git apply` 应用补丁
9. 运行测试（自动检测）
10. 如果测试失败则回滚补丁
11. 写入 `.dumbcoder/logs/` 作为审计日志

**安全保障**:
- 应用前验证补丁（`git apply --check`）
- 用户必须确认后才会应用
- 应用后自动运行测试
- 测试失败时自动回滚
- 所有操作均记录审计日志

## dumbcoder run

运行在 `.dumbcoder/plugins/*.toml` 中定义的插件命令。

```bash
dumbcoder run security-audit "审计认证模块"
dumbcoder run doc-gen "为订单服务生成文档"
```

**插件文件格式** (`.dumbcoder/plugins/<name>.toml`):

```toml
name = "security-audit"
description = "审计代码中的安全漏洞"
system_prompt = """
你是一个安全审计员。分析代码上下文中的安全漏洞。
重点关注：SQL 注入、XSS、路径遍历、认证绕过、代码中的密钥泄露。
输出按严重等级（严重/高/中/低）分类的结构化报告。
"""
```

**流程**:
1. 从 `.dumbcoder/plugins/` 加载插件
2. 按名称查找插件
3. 搜索代码库（ripgrep + 索引）
4. 组装代码上下文
5. 使用插件的 `system_prompt` 调用模型
6. 输出结果

## Prompt 模板覆盖

在 `.dumbcoder/config.toml` 中覆盖内置命令的 prompt：

```toml
[prompts]
ask = "你是一个 Rust 专家。用 Rust 惯用法回答问题。"
review = "你是一个严格的代码审查员，专注于性能。"
```

支持的键：`ask`、`explain`、`review`、`test`、`patch`
