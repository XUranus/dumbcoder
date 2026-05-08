# Security Design

## Core Principles

dumbcoder follows these security principles:

1. **Read-only by default** — The tool only reads code and generates suggestions; it does not write files directly
2. **Patch-first** — All modifications must generate a diff first, applied only after user confirmation
3. **Human confirmation** — Critical operations require developer approval
4. **Full audit trail** — All operations are logged

## File Access Control

### Blacklisted Directories

Files in these directories are never indexed or read:

```
.git
target
node_modules
dist
build
__pycache__
.dumbcoder
```

### Blacklisted Files

These files are never indexed or read:

```
.env, .env.local, .env.production
*.pem, *.key
id_rsa, id_ed25519
credentials.*
secrets.*
```

### Blacklisted Extensions

```
.pem, .key, .p12, .pfx, .jks
```

### Path Sandbox

The tool can only access files within the project root directory. Access outside the project directory is denied.

## Command Whitelist

### Default Allowed Commands

```
rg
git status
git diff
git log
git show
```

### Default Denied Commands

```
rm, mv, chmod, chown
ssh, scp, curl, wget
kubectl, docker
mysql, psql, redis-cli
Deployment scripts, production scripts
```

## Patch Safety

All code modifications must follow this flow:

```
AI generates modification suggestion
    ↓
Generate unified diff
    ↓
git apply --check (validate)
    ↓
User confirmation
    ↓
Apply patch
    ↓
Run tests
```

The model is never allowed to directly overwrite source files.

## Audit Log

The following are recorded:
1. User commands
2. List of files read
3. Tools called
4. Model request summary
5. Generated answers
6. Generated diffs
7. Whether patch was applied
8. Test results
9. Error messages

Logs are stored in `.dumbcoder/logs/`.
