# 上游同步指南

本项目是 [openai/codex](https://github.com/openai/codex) 的社区分支。为保持功能完整性和安全性，我们需要定期同步上游更新。

## 重要性

> [!IMPORTANT]
> 定期同步上游仓库是本项目可持续发展的关键。上游仓库会不断修复 bug、改进性能、增加新功能，我们需要及时合并这些改进。

## 同步原则

1. **保持特色功能**：激活器集成、中文文档等本项目特有功能需在合并时妥善保留
2. **定期检查**：建议每周检查一次上游更新
3. **谨慎合并**：合并前充分测试，确保不破坏现有功能

## 同步步骤

### 1. 添加上游远程（首次）

```bash
git remote add upstream https://github.com/openai/codex.git
git fetch upstream
```

### 2. 查看上游更新

```bash
# 获取最新上游代码
git fetch upstream

# 查看上游提交历史
git log upstream/main --oneline -20

# 对比差异
git diff main..upstream/main --stat
```

### 3. 合并上游更新

```bash
# 确保在 main 分支
git checkout main

# 合并上游（推荐使用 rebase 保持历史清晰）
git rebase upstream/main

# 或使用 merge（保留合并历史）
git merge upstream/main
```

### 4. 处理冲突

常见冲突文件：
- `README.md` - 我们有自己的中文版本
- `docs/*.md` - 我们有中文文档和激活器相关文档
- `config.toml.example` - 我们有自定义配置

处理原则：
- **保留我们的特色内容**（激活器集成、中文文档）
- **采纳上游的功能改进**（新功能、bug 修复）
- **合并新增的文档内容**

### 5. 验证与推送

```bash
# 构建验证
./build-fast.sh

# 功能测试
code exec "echo test"

# 推送到我们的仓库
git push origin main
```

## 需要保护的特色文件

以下文件需要在合并时特别注意保留我们的修改：

| 文件 | 我们的特色 |
|------|-----------|
| `README.md` | 激活器集成章节、中文内容 |
| `docs/activator-integration.md` | 完全是我们的文档 |
| `docs/command-reference.md` | 中文命令速查表 |
| `docs/authentication.md` | 激活器相关章节 |
| `config.toml.example` | 深色主题、激活器配置 |
| `codex-rs/core/src/auth.rs` | 第三方提供商注释 |
| `codex-rs/core/src/model_provider_info.rs` | 第三方提供商注释 |

## 自动化（可选）

可以设置 GitHub Actions 定期检查上游更新并创建 PR：

```yaml
# .github/workflows/upstream-sync.yml
name: Sync with Upstream

on:
  schedule:
    - cron: '0 0 * * 0'  # 每周日检查
  workflow_dispatch:

jobs:
  sync:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Fetch upstream
        run: |
          git remote add upstream https://github.com/openai/codex.git
          git fetch upstream
      - name: Check for updates
        run: |
          if git diff --quiet main..upstream/main; then
            echo "No updates"
          else
            echo "Updates available"
            git log upstream/main --oneline -10
          fi
```

## 相关链接

- 上游仓库：https://github.com/openai/codex
- 上游更新日志：https://github.com/openai/codex/releases
