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

## 即将推出

- `dumbcoder test` — 生成单元测试
- `dumbcoder review` — 审查 Git diff
- `dumbcoder patch` — 生成受控代码修改
- `dumbcoder tui` — 交互式 TUI
