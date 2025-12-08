# 认证

## 按量计费方案：使用 OpenAI API Key

如果你更喜欢按量计费，可以将 OpenAI API Key 设置为环境变量进行认证：

```shell
export OPENAI_API_KEY="your-api-key-here"
```

或从文件读取：

```shell
code login --with-api-key < my_key.txt
```

旧的 `--api-key` 参数现在会提示错误，要求改用 `--with-api-key`，以避免密钥出现在 shell 历史或进程列表中。

此密钥至少需要对 Responses API 的写入权限。

## 从 API Key 迁移到 ChatGPT 登录

如果之前用 API Key 方式按量付费，现在想切换到 ChatGPT 套餐，请按以下步骤：

1. 升级 CLI，确保 `code --version` 为 `0.5.0` 或更高
2. 删除 `~/.code/auth.json`（若存在旧版 `~/.codex/auth.json` 也一并删除；Windows 下路径为 `C:\\Users\\USERNAME\\.code\\auth.json` 与 `C:\\Users\\USERNAME\\.codex\\auth.json`）
3. 再次运行 `code login`

## 强制特定认证方式（高级）

当两种方式都存在时，你可以显式指定 Code 优先使用哪一种。

- 始终使用 API Key（即便已有 ChatGPT 登录）：

```toml
# ~/.code/config.toml（也会读取旧版 ~/.codex/config.toml）
preferred_auth_method = "apikey"
```

或在 CLI 临时覆盖：

```bash
code --config preferred_auth_method="apikey"
```

- 优先使用 ChatGPT 登录（默认）：

```toml
# ~/.code/config.toml（也会读取旧版 ~/.codex/config.toml）
preferred_auth_method = "chatgpt"
```

说明：

- 当 `preferred_auth_method = "apikey"` 且可用 API Key 时，会跳过登录界面。
- 当 `preferred_auth_method = "chatgpt"`（默认）时，存在 ChatGPT 登录则优先使用；若仅有 API Key 则使用 API Key。某些账号类型也可能要求 API Key 模式。
- 要查看会话中使用的认证方式，可在 TUI 里使用 `/status` 命令。

## 项目 .env 安全性（OPENAI_API_KEY）

默认情况下，Code 不再从项目的本地 `.env` 文件读取 `OPENAI_API_KEY` 或 `AZURE_OPENAI_API_KEY`。

原因：许多仓库会在 `.env` 里放与其他工具相关的 API Key，可能导致 Code 在该目录下静默使用 API Key 而非 ChatGPT 套餐。

仍然有效的来源：

- `~/.code/.env`（或 `~/.codex/.env`）最先加载，可放全局的 `OPENAI_API_KEY`。
- Shell 导出的 `OPENAI_API_KEY` 会被使用。

项目级 `.env` 中的提供商密钥一律忽略——没有可选开关。

UI 提示：

- 当 Code 使用 API Key 时，聊天页脚会显示醒目的 “Auth: API key” 徽章，便于辨识模式。

## 在“无头”机器上登录

目前登录流程会在 `localhost:1455` 启动一个服务器。若你在“无头”环境（如 Docker 容器或 SSH 登录的远程机器），本地浏览器打开 `localhost:1455` 不会自动连到该远程服务器，需要按以下方式之一处理：

### 在本地认证后复制凭据到无头机器

最简单的方式是在本地完成 `code login`（此时浏览器可访问 `localhost:1455`）。认证完成后，凭据会写入 `$CODE_HOME/auth.json`（默认 `~/.code/auth.json`；若存在仍会读取 `$CODEX_HOME`/`~/.codex/auth.json`）。

由于 `auth.json` 不绑定特定主机，完成本地认证后可将 `$CODE_HOME/auth.json` 复制到无头机器，`code` 就能直接使用。复制到 Docker 容器可这样做：

```shell
# 将 MY_CONTAINER 替换为容器名或 ID
CONTAINER_HOME=$(docker exec MY_CONTAINER printenv HOME)
docker exec MY_CONTAINER mkdir -p "$CONTAINER_HOME/.code"
docker cp auth.json MY_CONTAINER:"$CONTAINER_HOME/.code/auth.json"
```

如果是 SSH 到远程机器，通常使用 [`scp`](https://en.wikipedia.org/wiki/Secure_copy_protocol)：

```shell
ssh user@remote 'mkdir -p ~/.code'
scp ~/.code/auth.json user@remote:~/.code/auth.json
```

或者试试单行命令：

```shell
ssh user@remote 'mkdir -p ~/.code && cat > ~/.code/auth.json' < ~/.code/auth.json
```

### 通过 VPS 或远程机器登录

当在没有本地浏览器的远程机器（VPS/服务器）运行 Code 时，登录助手会在远程的 `localhost:1455` 启动服务器。要在本地浏览器完成登录，请在开始登录前把该端口转发到本机：

```bash
# 在本地机器执行
ssh -L 1455:localhost:1455 <user>@<remote-host>
```

然后在该 SSH 会话中运行 `code` 并选择 "Sign in with ChatGPT"。提示时，打开打印出的 URL（形如 `http://localhost:1455/...`）到本地浏览器，流量会被隧道到远程服务器。

## 第三方激活器集成

如果你使用第三方激活器（如 `codex-activator`）来管理认证，Code CLI 支持通过自定义 `model_providers` 配置来使用激活器提供的代理服务。

### 快速配置

激活器通常会将配置写入 `~/.codex/config.toml`。要让 Code CLI 使用相同配置，需要同步到 `~/.code/`：

```bash
# 手动同步
cp ~/.codex/config.toml ~/.code/config.toml

# 或设置自动同步（添加到 ~/.zshrc）
if [ -f ~/.codex/config.toml ]; then
  cp ~/.codex/config.toml ~/.code/config.toml 2>/dev/null
fi
```

### 工作原理

激活器配置示例：

```toml
model_provider = "crs"

[model_providers.crs]
name = "crs"
base_url = "https://proxy.example.com/openai"  # 代理服务器地址
wire_api = "responses"
requires_openai_auth = true
env_key = "CRS_OAI_KEY"  # 认证 token 的环境变量名
```

详细配置说明请参阅 [激活器集成指南](activator-integration.md)。
