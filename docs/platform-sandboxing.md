### 平台沙箱细节

Code 所用的沙箱机制因操作系统而异。

## macOS 12+
- 通过 `sandbox-exec` 使用 Apple Seatbelt，配置文件与所选 `--sandbox` 模式对应。

## Linux
- 使用 Landlock 加 seccomp 应用沙箱策略。
- 在容器环境（如 Docker）中需要宿主支持这些 API。若不支持，请让容器自身提供所需隔离，并在容器内以 `--sandbox danger-full-access`（或 `--dangerously-bypass-approvals-and-sandbox`）运行 Code。

## Windows
Code 用受限的 Windows token 启动命令，并基于声明的工作区根设置允许列表。在这些根以外（以及请求 workspace-write 时的 `%TEMP%`）写入会被阻止；常见逃逸方式如替代数据流、UNC 路径、设备句柄会被主动拒绝。CLI 还会在宿主 `PATH` 前插入 stub 可执行文件（如包装 `ssh`），以便在风险工具逃逸前拦截。

### 已知限制（smoketests）
运行 `python windows-sandbox-rs/sandbox_smoketests.py`，在完全文件系统与网络访问下目前通过 **37/41** 个用例，剩余高优先级问题：

| 测试 | 目的 |
| --- | --- |
| ADS write denied (#32) | 工作区内仍可写入替代数据流，应当阻止。 |
| Protected path case-variation denied (#33) | `.GiT` 绕过针对 `.git` 的保护，应拒绝大小写变体。 |
| PATH stub bypass denied (#35) | 将工作区的 `ssh.bat` 放到 PATH 首位时拦截不稳定，无法保证执行。 |
| Start-Process https denied (#41) | 只读运行中 `Start-Process 'https://…'` 仍成功，因为 Explorer 在沙箱外处理 ShellExecute。 |

### 如何贡献
若你能改进 Windows 沙箱，请针对上述四个 smoketest 失败项修复，并反复运行 `python windows-sandbox-rs/sandbox_smoketests.py`，直至 **41/41** 全部通过。
