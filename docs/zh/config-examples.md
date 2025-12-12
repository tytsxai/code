# 配置示例（中文）

> 主配置位于 `~/.code/config.toml`（兼容读取 `~/.codex/config.toml`）。下列示例以中文用户常见需求为导向，未修改默认行为。

## 基础示例：指定模型与审批策略

```toml
model = "gpt-5.2"
model_provider = "openai"
approval_policy = "on-request"  # 需要时询问审批
sandbox_mode = "workspace-write"  # 允许修改当前工作区
```

## 使用代理/镜像

在 shell 中设置（示例）：

```bash
export http_proxy=http://127.0.0.1:7890
export https_proxy=$http_proxy
```

若 npm 访问缓慢，可在 `~/.npmrc` 添加：

```
registry=https://registry.npmmirror.com
```

## 区分不同模型配置（Profiles）

```toml
[profiles.gpt5]
model = "gpt-5.2"
model_provider = "openai"
approval_policy = "never"
model_reasoning_effort = "high"
```

启动时通过 `--profile gpt5` 使用。

## MCP 服务器示例

```toml
[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/project"]
```

## 通知脚本（macOS 例）

```toml
[notify]
command = "terminal-notifier"
args = ["-title", "Codex", "-message", "{message}"]
```

## 语言与本地化

当前默认英文；可通过环境变量强制中文：

```bash
export CODEX_LANG=zh_CN.UTF-8
```

未设置时会自动读取 `LANG`，非中文 locale 将回退英文。
