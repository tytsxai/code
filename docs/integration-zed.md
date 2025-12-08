# Zed 集成

要让 Zed 连接 Every Code（Code）的 ACP 服务器，在 `settings.json` 中添加：

```jsonc
{
  "agent_servers": {
    "Code": {
      "command": "npx",
      "args": ["-y", "@just-every/code", "acp"]
    }
  }
}
```

仅在固定特定版本或使用全局安装的二进制时才需要调整 `command` 或 `args`。

## Zed 前置条件

- Zed Stable `0.201.5`（2025 年 8 月 27 日发布）或更新版本在 Agent Panel 中加入 ACP 支持。接入 Every Code 前请通过 `Zed → Check for Updates` 升级。Zed 文档说明 ACP 是驱动 Gemini CLI 及其他外部智能体的机制。
- 外部智能体位于 Agent Panel（`cmd-?`）内。点击 `+` 新建线程，从外部智能体列表选择 `Code`（Every Code）。Zed 以子进程方式通过 JSON‑RPC 运行我们的 CLI，提示与 diff 预览都保留在本地。
- Zed 会按条目自动安装依赖。如果保持 `command = "npx"`，Zed 会在首次触发集成时下载发布的 `@just-every/code` 包。

## Every Code 如何实现 ACP

- Rust MCP 服务器暴露 ACP 工具：`session/new`、`session/prompt` 以及通过 `session/cancel` 的快速中断。它们复用驱动 TUI 的对话管理器，因此审批、确认保护和沙箱策略保持一致。
- 流式的 `session/update` 通知将 Code 事件桥接到 Zed。你可以在 Zed UI 中看到 Answer/Reasoning 更新、命令进度、审批与 apply_patch diff，同时保持终端一致性。
- MCP 配置集中在 `CODE_HOME/config.toml`（也兼容读取 `CODEX_HOME/config.toml`）。使用 `[experimental_client_tools]` 可将文件读写和权限请求委托给 Zed，让其 UI 处理审批。最小示例如下：

```toml
[experimental_client_tools]
request_permission = { mcp_server = "zed", tool_name = "requestPermission" }
read_text_file = { mcp_server = "zed", tool_name = "readTextFile" }
write_text_file = { mcp_server = "zed", tool_name = "writeTextFile" }
```

添加 Code（Every Code）智能体时 Zed 会自动连接这些工具，上述标识符与默认值一致。
- CLI 入口（`npx @just-every/code acp`）是对 Rust 二进制（`cargo run -p code-mcp-server -- --stdio`）的薄封装，随 Every Code 一同发布。从源码构建时可将 `command` 换成该二进制的绝对路径。

## 提示与故障排查

- 需要查看握手？在命令面板运行 Zed 的 `dev: open acp logs`，日志会展示 JSON‑RPC 请求与 Code 响应。
- 如果提示卡住，确认没有其他进程占用同一 MCP 端口，且你的 `CODE_HOME`（或旧版 `CODEX_HOME`）指向期望的配置目录。ACP 服务器继承 Every Code 的沙箱设置，因此限制性策略（如 `approval_policy = "never"`）仍然生效。
- 目前 Zed 对第三方智能体跳过历史恢复与检查点 UI。如果依赖这些功能，请使用 TUI；ACP 支持仍在上游演进中。
- 会话开始后，Zed 的模型选择器会列出 Every Code 的内置预设（如 `gpt-5.1-codex`、`gpt-5.1` 高/中/低）。选择新预设会立即更新运行中的 Code 会话，无需重启智能体即可切换模型。
