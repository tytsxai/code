# 第三方激活器集成指南

本文档介绍如何将第三方激活器（如 `codex-activator`）与 Code CLI 集成，实现一次激活即可在多个版本间共享认证。

## 概述

第三方激活器通常使用自定义代理服务器提供 OpenAI API 兼容接口。Code CLI 支持通过 `model_providers` 配置来使用这些代理服务。

## 工作原理

1. **激活器**：将认证 token 写入环境变量（如 `CRS_OAI_KEY`），并配置 `~/.codex/config.toml`
2. **Code CLI**：读取 `~/.code/config.toml` 中的 `model_providers` 配置，使用指定的环境变量作为 API Key

## 配置同步

### 自动同步（推荐）

在 `~/.zshrc` 或 `~/.bashrc` 中添加以下内容，每次打开终端自动同步配置：

```bash
# 自动同步激活器配置到 ~/.code
# 当激活器更新 ~/.codex/config.toml 后，下次打开终端会自动同步到 ~/.code
if [ -f ~/.codex/config.toml ]; then
  cp ~/.codex/config.toml ~/.code/config.toml 2>/dev/null
fi
```

### 手动同步

每次更换激活码后手动执行：

```bash
cp ~/.codex/config.toml ~/.code/config.toml
```

## 配置示例

典型的激活器配置（`~/.code/config.toml`）：

```toml
# 指定使用的 model_provider
model_provider = "crs"

# 自定义 model_provider 定义
[model_providers.crs]
name = "crs"                                    # Provider 显示名称
base_url = "https://proxy.example.com/openai"   # 代理服务器地址
wire_api = "responses"                          # API 类型：responses 或 chat
requires_openai_auth = true                     # 是否需要认证
env_key = "CRS_OAI_KEY"                         # 认证 token 的环境变量名
```

### 配置字段说明

| 字段 | 说明 |
|------|------|
| `name` | Provider 的显示名称 |
| `base_url` | 代理服务器的 API 地址 |
| `wire_api` | API 协议类型，`responses`（OpenAI Responses API）或 `chat`（Chat Completions） |
| `requires_openai_auth` | 设为 `true` 表示需要认证 |
| `env_key` | 存储 API token 的环境变量名称 |

## 使用流程

### 首次设置

1. 安装激活器：
   ```bash
   npm i -g <activator-package>
   ```

2. 运行激活器并输入激活码：
   ```bash
   codex-activator
   ```

3. 同步配置（如果已设置自动同步，跳过此步）：
   ```bash
   cp ~/.codex/config.toml ~/.code/config.toml
   ```

4. 重新加载环境：
   ```bash
   source ~/.zshrc
   ```

5. 验证：
   ```bash
   code exec "echo hello"
   ```

### 更换激活码

1. 运行激活器并输入新激活码：
   ```bash
   codex-activator
   ```

2. 打开新终端（自动同步）或手动执行 `source ~/.zshrc`

3. 新配置自动生效

## 故障排查

### 认证失败

1. 检查环境变量是否设置：
   ```bash
   echo $CRS_OAI_KEY  # 或其他激活器使用的变量名
   ```

2. 检查配置文件是否同步：
   ```bash
   cat ~/.code/config.toml | grep model_provider
   ```

3. 确认 `env_key` 与实际环境变量名匹配

### 配置未生效

1. 确保已执行 `source ~/.zshrc`
2. 检查自动同步脚本是否在 `~/.zshrc` 中

## 安全注意事项

- 激活器的 token 存储在环境变量中，不会写入 `auth.json`
- `config.toml` 中不包含敏感信息，仅包含配置结构
- 建议定期更换激活码
