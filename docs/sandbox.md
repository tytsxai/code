## 沙箱与审批

### 审批模式

默认模式为 `Auto`：Code 可自动读取文件、修改并在工作目录运行命令，但在访问工作区外或联网时需你的批准。

如果只想聊天或先规划，可用 `/approvals` 切换到 `Read Only` 模式。

需要在无审批的情况下读取文件、修改并联网执行命令时，可使用 `Full Access`，请谨慎操作。

#### 默认与推荐

- Code 默认在沙箱中运行，带有严格护栏：阻止工作区外写入并默认禁用网络。
- 启动时会检测目录是否受版本控制并建议：
  - 受版本控制：`Auto`（workspace write + on-request approvals）
  - 非版本控制：`Read Only`
- 工作区包含当前目录与 `/tmp` 等临时目录。用 `/status` 查看哪些目录在工作区内。
- 可显式设置：
  - `code --sandbox workspace-write --ask-for-approval on-request`
  - `code --sandbox read-only --ask-for-approval on-request`

### 可以完全不弹审批吗？

可以。`--ask-for-approval never` 会关闭所有审批提示，与任意 `--sandbox` 组合使用。Code 会在你给定的约束下尽力完成任务。

### 常见沙箱 + 审批组合

| 意图                               | 参数                                                                                      | 效果                                                                                                             |
| ---------------------------------- | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| 安全只读浏览                       | `--sandbox read-only --ask-for-approval on-request`                                       | Code 可读文件并回答问题；修改、运行命令或联网需审批。                                                            |
| 只读的非交互（CI）                 | `--sandbox read-only --ask-for-approval never`                                            | 仅读取，从不升级。                                                                                               |
| 允许改动仓库，风险时再询问         | `--sandbox workspace-write --ask-for-approval on-request`                                 | 工作区内可读写并运行命令；工作区外或联网需审批。                                                                  |
| Auto 预设                          | `--full-auto`（等同 `--sandbox workspace-write` + `--ask-for-approval on-failure`）       | 工作区内可读写并运行；沙箱命令失败或需升级时才请求审批。                                                          |
| YOLO（不推荐）                     | `--dangerously-bypass-approvals-and-sandbox`（别名 `--yolo`）                             | 无沙箱、无提示。                                                                                                  |

> 注意：在 `workspace-write` 下默认禁用网络，除非在配置中开启（`[sandbox_workspace_write].network_access = true`）。

#### 在 `config.toml` 微调

```toml
# 审批模式
approval_policy = "untrusted"
sandbox_mode    = "read-only"

# 全自动
approval_policy = "on-request"
sandbox_mode    = "workspace-write"

# 可选：在 workspace-write 下允许网络
[sandbox_workspace_write]
network_access = true
```

也可用 **profiles** 保存预设：

```toml
[profiles.full_auto]
approval_policy = "on-request"
sandbox_mode    = "workspace-write"

[profiles.readonly_quiet]
approval_policy = "never"
sandbox_mode    = "read-only"
```

### 试验 Code 沙箱

想测试命令在 Code 沙箱下的行为，可使用 CLI 子命令：

```
# macOS
code sandbox macos [--full-auto] [COMMAND]...

# Linux
code sandbox linux [--full-auto] [COMMAND]...

# 旧别名
code debug seatbelt [--full-auto] [COMMAND]...
code debug landlock [--full-auto] [COMMAND]...
```

### 平台沙箱细节

沙箱机制取决于操作系统：

- **macOS 12+** 使用 **Apple Seatbelt**，通过 `sandbox-exec -p <profile>` 运行，与 `--sandbox` 对应。
- **Linux** 组合使用 Landlock/seccomp API。

在容器环境（如 Docker）中运行 Linux 时，若宿主/容器不支持所需 Landlock/seccomp API，沙箱可能不可用。此时建议为容器配置所需的隔离保障，并在容器内以 `--sandbox danger-full-access`（或 `--dangerously-bypass-approvals-and-sandbox`）运行 `code`。
