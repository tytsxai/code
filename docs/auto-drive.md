# Auto Drive

了解 Auto Drive 是什么、如何启动以及在 Every Code 中的行为。

## 起始方式
- TUI：`/auto <goal>`。若省略目标且存在近期历史，Code 会为你提议一个；`/auto settings` 可直接进入 Auto Drive 面板。
- CLI：`code exec --auto "<goal>"` 或 `code exec "/auto <goal>"`。无头模式必须提供目标。
- 前置条件：必须在 TUI 选择全自动模式（danger-full-access + approval=never），否则会看到警告且 Auto Drive 不会启动。

## 目标处理
- 在 CLI 传入的图片会在第一轮前附加。
- 如果未提供目标且历史中无法推导，Auto Drive 不会启动。

## 运行方式
- 每轮都会起草计划、准备命令、可选分配智能体，并在运行前等待你的确认（或倒计时结束）。
- 对话记录保存在内存中并自动压缩；若历史被裁剪，会显示提示。
- 若存在 `AUTO_AGENTS.md`，其指导会与 AGENTS.md 规则一起作用于本次运行。

## Agents
- Auto Drive 可以在一轮中启动辅助智能体。可在设置中的 `agents_enabled` 切换。
- 在非 git 仓库中，Auto Drive 会强制这些智能体以只读方式运行，避免意外写入。

## 观察者
- 轻量级观察者每隔 `auto_drive_observer_cadence` 轮（默认 5）审阅一次运行。发现问题会在横幅提示。将该值设为 `0` 可禁用。

## 沙箱与审批
- TUI：需要 `danger-full-access` 且 `approval_policy=never`，避免被审批阻塞。
- CLI：`--auto` 会关闭审批；如需允许修改与联网命令，添加 `--full-auto`（或 `--dangerously-bypass-approvals-and-sandbox`）。

## 继续与倒计时模式
- `continue_mode`：`immediate`、`ten-seconds`（默认）、`sixty-seconds`、`manual`。
- 在倒计时模式下，Auto Drive 卡片显示计时器；Enter 可提前继续，Esc 重新打开草稿，0 自动提交。
- 手动模式会在每个已准备的提示后暂停，等待你确认。

## 停止与暂停
- Auto Drive 活跃时按 Esc 可暂停或停止（取决于上下文）。倒计时模式会在页脚显示提示。
- 审批对话不会截获 Esc；始终传递给 Auto Drive。

## 审查、QA、诊断
- `review_enabled`（默认 true）可插入审查环节；卡片会显示 “Awaiting review”。
- `qa_automation_enabled` 与 `cross_check_enabled`（默认 true）允许继续前进行诊断与交叉检查。
- `auto_resolve_review_attempts` 限制自动解决审查反馈的次数（默认 5）。

## 模型
- 默认：模型 `gpt-5.1`，推理力度 `high`。
- 在设置中切换“use chat model”即可复用当前聊天模型/力度，而不是专用的 Auto Drive 模型。

## UI 展示
- Auto Drive 卡片显示状态（Ready、Waiting、Thinking、Running、Awaiting review、Failed/Stopped）、目标、动作日志、token/时间计数、倒计时以及成功时的庆祝效果。
- 底部面板标题会同步状态并显示提示（Ctrl+S 设置、Esc 停止、是否启用智能体/诊断）。

## 恢复与持久化
- 历史保存在内存中；没有 Auto Drive 专属历史文件。被裁剪时会提示。
- 你可以像平常一样恢复会话；Auto Drive 可从恢复的历史中推导目标。
- CLI 的 `--output-last-message` 依然可用，仅需要最终回复时可使用。

## 增强功能（实验性）

以下功能通过 `code-auto-drive-core` 模块提供，目前处于实验阶段：

### 检查点系统
- 自动保存会话状态，支持崩溃恢复
- 使用 SHA-256 校验和验证数据完整性
- 可配置保存间隔（默认每 5 轮）

### 诊断引擎
- 循环检测：识别重复的工具调用模式
- 目标偏离检测：监控上下文与原始目标的相关性
- Token 异常检测：当实际使用超过预估 50% 时告警

### 预算控制
- Token 预算：设置最大 token 使用量
- 轮次限制：限制最大执行轮数
- 时间限制：设置最大执行时长
- 80% 警告阈值，100% 自动暂停

### 智能体调度
- 并行执行：多智能体同时运行
- 阻塞执行：按顺序依次运行
- 可配置并发限制（默认 4）

### 审计日志
- 记录所有工具执行、文件修改、网络访问
- 支持 JSON 导出
- 工作区路径验证

### 遥测收集
- OpenTelemetry 兼容的 span 跟踪
- 会话和轮次级别的指标
- 错误记录和调试日志

### 智能历史压缩
- 语义感知：保留关键决策和错误
- 目标保护：始终保留原始目标
- 可配置保留策略

## 设置（config.toml）
- 顶层键：`auto_drive_use_chat_model`（默认 false）、`auto_drive_observer_cadence`（默认 5）。
- `[auto_drive]` 默认：`review_enabled=true`、`agents_enabled=true`、`qa_automation_enabled=true`、`cross_check_enabled=true`、`observer_enabled=true`、`coordinator_routing=true`、`continue_mode="ten-seconds"`、`model="gpt-5.1"`、`model_reasoning_effort="high"`、`auto_resolve_review_attempts=5`。
- 以上均可在 TUI 的 `/auto settings` 或直接在 `config.toml` 中修改。

## 小贴士
- 想要倒计时与可视状态请留在 TUI；CI 或脚本流程可用 `code exec --auto`。
- 如因无法推导目标而停止，请用简短具体的指令重新运行 `/auto <goal>`。
- 想要单模型运行可在 `/auto settings` 关闭智能体。
