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

## 配置文件

配置文件位于 `.dumbcoder/config.toml`：

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

## 即将推出

- `dumbcoder test` — 生成单元测试
- `dumbcoder review` — 审查 Git diff
- `dumbcoder patch` — 生成受控代码修改
- `dumbcoder tui` — 交互式 TUI
