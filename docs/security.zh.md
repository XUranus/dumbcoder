# 安全设计

## 核心原则

dumbcoder 遵循以下安全原则：

1. **默认只读** — 工具默认只允许读取代码和生成建议，不允许直接写文件
2. **Patch-first** — 所有修改必须先生成 diff，经用户确认后才应用
3. **人工确认** — 关键操作必须经过开发人员确认
4. **全程审计** — 所有操作记录日志

## 文件访问控制

### 黑名单目录

以下目录中的文件不会被索引或读取：

```
.git
target
node_modules
dist
build
__pycache__
.dumbcoder
```

### 黑名单文件

以下文件不会被索引或读取：

```
.env, .env.local, .env.production
*.pem, *.key
id_rsa, id_ed25519
credentials.*
secrets.*
```

### 黑名单扩展名

```
.pem, .key, .p12, .pfx, .jks
```

### 路径沙箱

工具只能访问项目根目录下的文件，不能访问项目目录外的文件。

## 命令白名单

### 默认允许的命令

```
rg
git status
git diff
git log
git show
```

### 默认禁止的命令

```
rm, mv, chmod, chown
ssh, scp, curl, wget
kubectl, docker
mysql, psql, redis-cli
部署脚本、生产环境脚本
```

## Patch 安全

所有代码修改必须经过以下流程：

```
AI 生成修改建议
    ↓
生成 unified diff
    ↓
git apply --check（校验）
    ↓
用户确认
    ↓
应用 patch
    ↓
运行测试
```

不允许模型直接覆盖源文件。

## 审计日志

记录以下内容：
1. 用户命令
2. 读取文件列表
3. 调用工具列表
4. 模型请求摘要
5. 生成回答
6. 生成 diff
7. 是否应用 patch
8. 测试结果
9. 错误信息

日志保存在 `.dumbcoder/logs/` 目录。
