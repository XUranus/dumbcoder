下面是修改后的版本，定位改为：**基于 Rust 开发的内网 AI Coding Assistant，优先提供 CLI / TUI 工具形态**。

---

# 基于 Rust CLI / TUI 的内网 AI Coding Assistant 建设 Proposal

## 一、项目背景

公司当前软件开发环境受内网安全限制，无法直接使用 Cursor、GitHub Copilot、Claude Code、ChatGPT Codex 等外部 AI 编程工具。但研发团队仍然希望利用 AI 提升代码理解、问题定位、测试生成、文档生成和代码审查效率。

同时，公司现有硬件资源主要是消费级显卡，无法部署超大规模模型，只能稳定运行 Qwen2.5-Coder-7B 等 7B 级别代码模型。因此，本项目需要在有限算力和有限模型能力条件下，建设一套 **内网可部署、数据不出域、轻量高效、安全可控** 的 AI Coding Assistant。

考虑到公司内网开发场景通常以终端、Git、脚本、CI、服务器环境为主，本项目计划优先采用 **Rust 语言开发 CLI / TUI 工具**，而不是一开始建设复杂 Web 平台或 IDE 插件。

Rust 具备以下优势：

1. 性能高，适合代码索引、文件扫描、命令执行和本地工具开发。
2. 单二进制分发，部署简单，适合内网环境。
3. 安全性高，适合实现命令白名单、权限控制和沙箱执行。
4. 适合开发跨平台 CLI / TUI 工具。
5. 与 Git、ripgrep、tree-sitter、LSP、测试命令等开发工具集成方便。

---

## 二、项目定位

本项目不是建设一个完全自主开发的 AI 程序员，也不是复制 Claude Code、Devin 这类全自动 Agent。

在当前内网限制和消费级显卡资源条件下，本项目的合理定位是：

> 基于 Rust 开发一个轻量、安全、可审计的内网 AI Coding Assistant CLI / TUI 工具，帮助研发人员在终端中完成代码问答、代码解释、代码检索、测试生成、文档生成、PR Review 和小范围 Patch 辅助。

系统形态优先是：

```text
dumbcoder ask       # 代码库问答
dumbcoder explain   # 解释文件/函数/代码片段
dumbcoder search    # 智能代码搜索
dumbcoder test      # 生成/补充单元测试
dumbcoder review    # 审查 git diff
dumbcoder patch     # 生成受控代码修改 diff
dumbcoder tui       # 进入交互式 TUI
```

项目核心原则：

> 小模型负责理解、总结和生成建议；Rust 工具链负责代码检索、上下文组织、命令控制、测试验证和权限约束；开发人员负责最终确认和应用修改。

---

## 三、建设目标

### 3.1 总体目标

建设一个运行在公司内网环境中的 Rust CLI / TUI AI Coding Assistant，通过 Qwen2.5-Coder-7B 等本地小模型，结合代码索引、RAG、AST、LSP、测试工具和严格权限控制，提升研发团队日常开发效率。

### 3.2 具体目标

1. **内网部署**

   所有模型推理、代码索引、RAG 检索、命令执行和对话记录均在公司内网完成，不依赖外部 API。

2. **Rust 单二进制分发**

   主体工具基于 Rust 开发，支持编译为单个可执行文件，便于在内网开发机、服务器和 CI 环境中分发。

3. **CLI 优先**

   支持开发人员在终端中直接使用，方便接入 Git、测试命令、CI 流程和日常开发工作流。

4. **TUI 增强交互体验**

   提供终端交互界面，用于浏览 AI 回答、引用文件、代码片段、diff、测试结果和审查报告。

5. **适配小模型能力**

   面向 Qwen2.5-Coder-7B 等小模型设计任务边界，避免长链复杂自主开发，优先支持高频、局部、可验证任务。

6. **强化代码上下文能力**

   通过 ripgrep、tree-sitter、LSP、向量检索、BM25 和 Git 历史，为小模型提供精准上下文。

7. **保证安全可控**

   使用文件黑名单、命令白名单、patch-first、人工确认、审计日志等机制，避免 AI 误操作。

---

## 四、为什么选择 Rust CLI / TUI

### 4.1 适合内网开发环境

内网环境通常存在以下特点：

1. 无法访问外部 SaaS。
2. 开发人员习惯使用终端、Git、SSH、脚本和 CI。
3. 部署审批严格，不适合快速上线复杂 Web 服务。
4. 不同团队开发环境不统一。
5. 内网软件分发成本高。

Rust CLI 工具可以降低部署复杂度：

```text
构建一次
   ↓
生成单个二进制文件
   ↓
分发到开发机 / 跳板机 / CI 服务器
   ↓
配置模型服务地址和仓库路径
   ↓
直接使用
```

相比 Web 平台，CLI / TUI 的初期落地成本更低，也更贴近开发人员日常工作流。

---

### 4.2 适合工具链集成

Rust 很适合做本地开发工具，可以直接集成：

```text
git
rg
tree-sitter
LSP
cargo test
go test
pytest
npm test
git diff
git apply --check
```

这类工具调用和文件操作是 Coding Assistant 的核心能力，Rust 在这方面比纯 Web 应用更直接、更稳定。

---

### 4.3 适合安全控制

由于 AI Coding Assistant 需要读取代码、调用命令、生成 diff，安全控制非常重要。

Rust 可以更好地实现：

1. 命令白名单。
2. 路径沙箱。
3. 超时控制。
4. 输出截断。
5. 文件访问黑名单。
6. patch 校验。
7. 审计日志。
8. 权限分级。

---

### 4.4 适合后续扩展

初期可以做 CLI / TUI，后续可扩展为：

```text
Rust CLI
   ↓
Rust TUI
   ↓
本地守护进程 daemon
   ↓
内网模型网关
   ↓
Web UI
   ↓
IDE 插件
   ↓
CI / GitLab Bot
```

CLI / TUI 可以作为整个内网 AI Coding 平台的底座，而不是一次性建设大而全系统。

---

## 五、系统架构

### 5.1 总体架构

```text
┌──────────────────────────────────────────────┐
│              Rust CLI / TUI                  │
│  ask / explain / search / test / review / patch│
└──────────────────────────────────────────────┘
                     │
                     v
┌──────────────────────────────────────────────┐
│           Agent Orchestrator                 │
│  任务拆分 / 上下文管理 / 工具调用 / 安全策略   │
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
│ 文件权限 / 命令白名单 / patch 审计 / 日志       │
└──────────────────────────────────────────────┘
```

---

## 六、Rust 技术选型

### 6.1 CLI 框架

推荐使用：

```text
clap
```

用途：

1. 定义命令行参数。
2. 支持子命令。
3. 支持配置文件。
4. 自动生成帮助信息。

示例命令：

```bash
dumbcoder ask "订单取消逻辑在哪里？"
dumbcoder explain src/order/service.rs
dumbcoder review --staged
dumbcoder patch "修复用户为空时 panic"
```

---

### 6.2 TUI 框架

推荐使用：

```text
ratatui + crossterm
```

用途：

1. 终端交互界面。
2. 展示对话历史。
3. 展示代码片段。
4. 展示文件引用。
5. 展示 diff。
6. 展示测试结果。
7. 支持键盘操作。

TUI 界面结构可设计为：

```text
┌────────────────────────────────────────────┐
│ dumbcoder tui                                 │
├─────────────────────┬──────────────────────┤
│ Chat / Task          │ Context Files         │
│                      │ src/order/service.rs  │
│ 用户问题 / AI回答     │ src/order/repo.rs     │
├─────────────────────┴──────────────────────┤
│ Code / Diff / Test Result                   │
│                                              │
├────────────────────────────────────────────┤
│ Input: explain cancel_order                 │
└────────────────────────────────────────────┘
```

---

### 6.3 HTTP 客户端

推荐使用：

```text
reqwest
```

用途：

1. 调用本地模型服务。
2. 支持 Ollama API。
3. 支持 OpenAI-compatible API。
4. 支持内网模型网关。

---

### 6.4 异步运行时

推荐使用：

```text
tokio
```

用途：

1. 异步调用模型服务。
2. 并发读取文件。
3. 并发构建索引。
4. 执行命令并捕获输出。
5. 支持超时控制。

---

### 6.5 配置管理

推荐使用：

```text
serde + toml
```

配置文件示例：

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
allow = [
  "rg",
  "git",
  "cargo test",
  "go test",
  "pytest",
  "npm test",
  "tsc"
]
```

---

### 6.6 代码解析

推荐使用：

```text
tree-sitter
tree-sitter-rust
tree-sitter-go
tree-sitter-python
tree-sitter-typescript
tree-sitter-java
```

用途：

1. 函数级切分。
2. 类和方法识别。
3. import 分析。
4. 测试函数识别。
5. 局部上下文提取。
6. AST 索引构建。

---

### 6.7 全文检索

推荐使用：

```text
ripgrep
tantivy
```

其中：

1. `ripgrep` 适合实时搜索。
2. `tantivy` 适合构建本地全文索引。

初期可直接调用 `rg`，后续再引入 `tantivy` 做持久化索引。

---

### 6.8 向量检索

第一阶段可以不复杂化，优先实现关键词和 AST 检索。

第二阶段可引入：

```text
Qdrant
FAISS
sqlite-vec
LanceDB
```

如果希望保持 Rust 单体工具风格，可以优先考虑：

```text
sqlite + sqlite-vec
```

优点：

1. 部署简单。
2. 数据文件本地化。
3. 不需要额外服务。
4. 适合 CLI 工具。

---

### 6.9 LSP 集成

后续可接入：

```text
rust-analyzer
gopls
pyright
typescript-language-server
jdtls
clangd
```

用途：

1. 查找定义。
2. 查找引用。
3. 获取类型信息。
4. 获取诊断错误。
5. 结合 AI 解释诊断结果。

---

## 七、核心命令设计

### 7.1 `dumbcoder init`

初始化项目索引和配置。

```bash
dumbcoder init
```

功能：

1. 生成 `.dumbcoder/config.toml`。
2. 识别项目语言。
3. 检测 Git 仓库。
4. 检测可用测试命令。
5. 初始化索引目录。

---

### 7.2 `dumbcoder index`

构建或更新代码索引。

```bash
dumbcoder index
dumbcoder index --full
dumbcoder index --changed
```

功能：

1. 扫描代码文件。
2. 建立文件索引。
3. 建立符号索引。
4. 建立 AST 函数索引。
5. 可选建立向量索引。
6. 跳过敏感文件和忽略目录。

---

### 7.3 `dumbcoder ask`

代码库问答。

```bash
dumbcoder ask "订单取消逻辑在哪里？"
```

输出：

```text
相关文件：
1. src/order/service.rs
2. src/order/repository.rs
3. src/api/order_controller.rs

结论：
订单取消逻辑主要在 cancel_order 函数中实现。
该函数会检查订单状态，然后调用 repository 更新订单状态为 Cancelled。

依据：
- src/order/service.rs:42-88
- src/order/repository.rs:120-146
```

---

### 7.4 `dumbcoder explain`

解释文件、函数或代码片段。

```bash
dumbcoder explain src/order/service.rs
dumbcoder explain src/order/service.rs --symbol cancel_order
```

功能：

1. 提取相关代码。
2. 分析函数职责。
3. 总结输入输出。
4. 说明边界条件。
5. 提示潜在风险。

---

### 7.5 `dumbcoder search`

智能代码搜索。

```bash
dumbcoder search "订单状态流转"
dumbcoder search "cancel order status"
```

功能：

1. 关键词搜索。
2. 符号搜索。
3. 语义搜索。
4. AST 结果排序。
5. 返回相关文件和代码片段。

---

### 7.6 `dumbcoder test`

生成或补充单元测试。

```bash
dumbcoder test src/order/service.rs --symbol cancel_order
```

功能：

1. 分析目标函数。
2. 查找已有测试。
3. 生成测试建议。
4. 输出测试代码 diff。
5. 可选运行测试命令。

---

### 7.7 `dumbcoder review`

审查 Git diff。

```bash
dumbcoder review --staged
dumbcoder review --diff main...HEAD
```

功能：

1. 读取 git diff。
2. 分析变更风险。
3. 检查测试覆盖。
4. 提示潜在 bug。
5. 输出 review 报告。

输出示例：

```text
Review Summary

1. src/order/service.rs
   风险：中
   问题：新增 cancel_order 分支未处理 order_id 为空字符串的情况。
   建议：增加输入校验，并补充单元测试。

2. src/order/repository.rs
   风险：低
   问题：SQL 更新逻辑未检查 affected rows。
   建议：当 affected rows = 0 时返回 NotFound。
```

---

### 7.8 `dumbcoder patch`

生成受控代码修改 diff。

```bash
dumbcoder patch "修复 cancel_order 在 order_id 为空时 panic"
```

流程：

```text
1. 搜索相关代码
2. 读取上下文
3. 生成修改计划
4. 生成 unified diff
5. 执行 git apply --check
6. 用户确认是否应用
7. 可选运行测试
```

默认只输出 diff，不直接修改文件。

---

### 7.9 `dumbcoder tui`

进入交互式 TUI 模式。

```bash
dumbcoder tui
```

TUI 支持：

1. 多轮对话。
2. 浏览上下文文件。
3. 查看代码片段。
4. 查看 diff。
5. 选择是否应用 patch。
6. 运行测试。
7. 查看历史任务。

---

## 八、安全设计

### 8.1 默认只读

工具默认只允许读取代码和生成建议，不允许直接写文件。

默认行为：

```text
ask       只读
explain   只读
search    只读
review    只读
test      默认只生成 diff
patch     默认只生成 diff
```

只有用户明确确认后，才允许应用 patch。

---

### 8.2 文件访问控制

默认忽略：

```text
.git
target
node_modules
dist
build
.env
*.pem
*.key
id_rsa
id_ed25519
credentials.*
secrets.*
config/production.*
```

敏感文件既不进入索引，也不允许被 AI 读取。

---

### 8.3 命令白名单

默认允许：

```text
rg
git status
git diff
git log
git show
cargo test
cargo check
go test
pytest
npm test
pnpm test
tsc --noEmit
```

默认禁止：

```text
rm
mv
chmod
chown
ssh
scp
curl
wget
kubectl
docker
mysql
psql
redis-cli
部署脚本
生产环境脚本
```

---

### 8.4 Patch-first 机制

所有修改必须先生成 diff：

```text
AI 生成修改建议
        ↓
生成 unified diff
        ↓
git apply --check
        ↓
用户确认
        ↓
应用 patch
        ↓
运行测试
```

不允许模型直接覆盖源文件。

---

### 8.5 审计日志

记录内容：

1. 用户命令。
2. 读取文件列表。
3. 调用工具列表。
4. 模型请求摘要。
5. 生成回答。
6. 生成 diff。
7. 是否应用 patch。
8. 测试结果。
9. 错误信息。

日志默认保存在：

```text
.dumbcoder/logs/
```

---

## 九、实施计划

### 阶段一：Rust CLI MVP

周期建议：2 到 4 周。

目标：完成最小可用 CLI 工具。

功能：

1. `dumbcoder init`
2. `dumbcoder ask`
3. `dumbcoder explain`
4. `dumbcoder search`
5. 调用本地 Qwen2.5-Coder-7B
6. 调用 `rg` 搜索代码
7. 基础上下文拼接
8. 配置文件支持

验收标准：

1. 可以在内网开发机运行。
2. 可以连接本地 Ollama / llama.cpp 模型服务。
3. 可以对一个真实仓库进行代码问答。
4. 可以解释指定文件或函数。
5. 回答能返回相关文件路径。

---

### 阶段二：代码索引与 AST

周期建议：4 到 6 周。

目标：提升代码定位和上下文准确率。

功能：

1. `dumbcoder index`
2. 文件索引。
3. 符号索引。
4. tree-sitter 函数级切分。
5. AST 上下文提取。
6. Git changed files 增量索引。
7. 本地索引数据库。

验收标准：

1. 能识别函数、类、方法。
2. 能按 symbol 查询代码。
3. 能定位相关函数代码块。
4. 检索结果明显优于纯 grep。

---

### 阶段三：TUI 交互界面

周期建议：3 到 5 周。

目标：提升终端交互体验。

功能：

1. `dumbcoder tui`
2. 对话区。
3. 文件引用区。
4. 代码片段区。
5. diff 预览区。
6. 测试结果区。
7. 快捷键操作。
8. 历史任务浏览。

验收标准：

1. 开发人员可以在 TUI 中完成代码问答。
2. 可以查看 AI 引用的文件和代码片段。
3. 可以浏览 diff。
4. 可以确认或拒绝 patch。

---

### 阶段四：测试生成与 Review

周期建议：4 到 6 周。

目标：进入真实开发工作流。

功能：

1. `dumbcoder test`
2. `dumbcoder review`
3. 单元测试生成。
4. git diff review。
5. 测试命令识别。
6. 测试执行和日志摘要。
7. Review 报告生成。

验收标准：

1. 可以针对函数生成测试建议。
2. 可以审查 staged diff。
3. 可以发现基础风险。
4. 可以输出结构化 review 报告。

---

### 阶段五：受控 Patch Agent

周期建议：6 到 8 周。

目标：支持小范围代码修改。

功能：

1. `dumbcoder patch`
2. 修改计划生成。
3. diff 生成。
4. `git apply --check`
5. 用户确认应用。
6. 自动运行测试。
7. 失败回滚。
8. 审计日志。

验收标准：

1. 对明确小 bug 能生成可用 diff。
2. 所有修改必须经过人工确认。
3. 测试失败时能给出原因说明。
4. 不执行高危命令。
5. 操作全程可追踪。

---

## 十、MVP 推荐范围

第一版不建议做得过大，建议只实现以下能力：

```text
dumbcoder init
dumbcoder ask
dumbcoder explain
dumbcoder search
```

第一版暂不做：

```text
自动修改代码
自动应用 patch
复杂多轮 agent
IDE 插件
Web UI
多用户权限系统
```

MVP 目标是验证：

1. Qwen2.5-Coder-7B 在公司真实代码库中的可用性。
2. Rust CLI 工具是否符合开发人员习惯。
3. 简单代码检索 + 本地模型是否能带来效率提升。
4. 后续是否值得继续投入 AST、TUI、Review 和 Patch 能力。

---

## 十一、预期收益

项目落地后，预期收益包括：

1. **降低代码理解成本**

   开发人员可以直接在终端中询问项目结构、函数职责和调用关系。

2. **提升问题定位效率**

   AI 可以结合代码搜索和上下文分析，辅助定位 bug 来源。

3. **减少重复劳动**

   自动生成测试、文档、注释和 review 建议。

4. **贴合内网开发环境**

   CLI / TUI 工具不依赖浏览器和外部服务，更适合内网服务器和终端场景。

5. **部署维护简单**

   Rust 单二进制工具便于分发、升级和版本管理。

6. **安全可控**

   通过命令白名单、文件黑名单、patch-first 和审计日志控制风险。

7. **可逐步演进**

   后续可以从 CLI 扩展到 TUI、daemon、Web UI、IDE 插件和 GitLab Bot。

---

## 十二、风险与应对

### 风险一：Qwen2.5-Coder-7B 能力不足

表现：

1. 多文件复杂任务不稳定。
2. 长上下文理解能力有限。
3. 生成代码可能不完整。

应对：

1. 限制任务范围。
2. 使用检索增强。
3. 使用 AST 精准提取上下文。
4. 将复杂任务拆成小步骤。
5. 后续支持 14B / 32B 模型。

---

### 风险二：CLI 工具用户体验不如 IDE

表现：

1. 部分开发人员更习惯图形界面。
2. 长回答和 diff 在终端中阅读不方便。

应对：

1. 提供 TUI 模式。
2. 支持 Markdown 输出。
3. 支持导出报告。
4. 后续扩展 IDE 插件。

---

### 风险三：检索不准确

表现：

1. 找错文件。
2. 漏掉关键上下文。
3. 回答依据不足。

应对：

1. 结合 rg、AST、BM25、向量检索。
2. 回答必须附带文件路径。
3. 展示引用代码片段。
4. 支持用户手动指定文件和 symbol。

---

### 风险四：AI 误改代码

表现：

1. 生成错误 diff。
2. 引入新 bug。
3. 破坏现有逻辑。

应对：

1. 默认只读。
2. patch-first。
3. git apply --check。
4. 人工确认。
5. 自动测试。
6. 支持回滚。

---

## 十三、结论

在公司内网限制和消费级显卡资源条件下，直接建设完整自主型 AI Coding Agent 并不现实。但基于 Qwen2.5-Coder-7B 等小模型，结合 Rust CLI / TUI、本地代码索引、RAG、AST、LSP、测试工具和严格权限控制，建设一个 **内网 AI Coding Assistant** 是可行且有价值的。

本项目建议采用 Rust 作为核心开发语言，优先实现 CLI 工具，再逐步扩展 TUI、测试生成、PR Review 和受控 Patch 能力。

推荐落地路径是：

```text
Rust CLI MVP
   ↓
代码索引 + AST
   ↓
TUI 交互界面
   ↓
测试生成 + Review
   ↓
受控 Patch Agent
   ↓
平台化 / IDE / Git 集成
```

最终目标是形成一个轻量、安全、可审计、可扩展的内网 AI 编程辅助工具，在不泄露代码和业务数据的前提下，帮助公司研发团队获得可持续的 AI 编程能力。
