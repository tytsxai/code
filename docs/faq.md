## FAQ

### OpenAI 在 2021 年发布了名为 Codex 的模型——这和它有关吗？

只是同名。2021 年的 Codex 模型已在 2023 年 3 月下线。Every Code 是 `openai/codex` CLI 的社区分支，独立演进。

### 支持哪些模型？

推荐使用内置的 Code 预设（基于 GPT-5.1，如 `code-gpt-5.1-codex-max`）。默认推理级别为 medium，如需处理复杂任务可在 `/model` 升级到 high。

你也可以使用旧模型：采用 API 认证，并在启动 Code 时传入 `--model`。

### 为什么 `o3` 或 `o4-mini` 不可用？

你的 [API 账号可能需要验证](https://help.openai.com/en/articles/10910291-api-organization-verification) 才能开始流式输出并看到链路推理摘要。如果仍有问题，请告知我们！

### 如何阻止 Code 修改我的文件？

默认情况下，Code 处于 Auto 模式，会修改当前工作目录的文件。要阻止修改，可使用 CLI 参数 `--sandbox read-only` 以只读模式运行 `code`。也可在对话中通过 `/approvals` 调整审批级别。

### Windows 能用吗？

直接在 Windows 上运行可能可行，但未正式支持。推荐使用 [Windows Subsystem for Linux (WSL2)](https://learn.microsoft.com/en-us/windows/wsl/install)。

### 为什么 Code 在 Windows 上找不到我的 agents？

在 Windows 上，PATH 配置和文件扩展名会影响智能体发现。如果看到 `Agent 'xyz' could not be found` 之类的报错，试试以下方案：

**1. 使用绝对路径（推荐）**

在 `~/.code/config.toml` 中为智能体可执行文件写入完整路径：

```toml
[[agents]]
name = "claude"
command = "C:\\Users\\YourUser\\AppData\\Roaming\\npm\\claude.cmd"
enabled = true

[[agents]]
name = "gemini"
command = "C:\\Users\\YourUser\\AppData\\Roaming\\npm\\gemini.cmd"
enabled = true
```

将 `YourUser` 替换为你的 Windows 用户名。

**2. 找到 npm 全局安装位置**

运行以下命令查看 npm 全局安装路径：
```cmd
npm config get prefix
```

可执行文件位于返回的目录下。例如返回 `C:\Users\YourUser\AppData\Roaming\npm`，则智能体命令在：
- `C:\Users\YourUser\AppData\Roaming\npm\claude.cmd`
- `C:\Users\YourUser\AppData\Roaming\npm\gemini.cmd`
- `C:\Users\YourUser\AppData\Roaming\npm\coder.cmd`

**3. 确认 PATH 包含 npm 目录**

在 PowerShell：
```powershell
$env:PATH -split ';' | Select-String "npm"
```

在命令提示符：
```cmd
echo %PATH% | findstr npm
```

如果 PATH 中没有 npm 目录，可以：
- 将其加入系统 PATH（需重启），或
- 在配置中使用绝对路径（推荐）。

**4. 检查文件扩展名**

在 Windows 上，Code 会查找扩展名为 `.exe`、`.cmd`、`.bat`、`.com` 的可执行文件。使用绝对路径时确保命令包含正确扩展名。

**相关**：更多细节见 [Agent 配置指南](https://github.com/just-every/code/blob/main/code-rs/config.md#agents)。
