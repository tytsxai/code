# 架构与数据流总览

面向新贡献者的速览，概括 workspace 结构、请求/事件流水、可扩展点与高风险注意事项。

## Workspace / crate 地图

```
repo root
├─ code-rs/        # 可修改的 Rust workspace（主开发）
│  ├─ cli/         # 多入口 CLI，解析子命令并调度 TUI/exec/Auto Drive
│  ├─ tui/         # Ratatui 全屏界面，历史渲染、流式排序、审批交互
│  ├─ core/        # 会话/工具执行内核，生成 EventMsg 供 UI 消费
│  ├─ exec/        # 无头执行/Auto Drive headless 路径
│  ├─ code-auto-drive-core/  # Auto Drive 协调器与状态机
│  ├─ app-server, app-server-protocol # MCP/JSON-RPC 桥接
│  ├─ protocol, protocol-ts  # 内部 SQ/EQ 协议与 TS 绑定
│  ├─ browser/     # CDP/headless 浏览器管理
│  ├─ agents/tooling (agent_tool 在 core) # 子智能体生命周期与模型校验
│  ├─ cloud-tasks* # 远端任务列取/应用
│  ├─ otel/        # 遥测事件聚合与导出
│  └─ 其他支撑 crate（git-apply、apply-patch、file-search、execpolicy 等）
└─ codex-rs/       # 上游镜像，只读（对比/同步用）
```

## 主数据流（请求 / 响应）

```mermaid
flowchart LR
  subgraph UI
    CLI[cli
    clap 入口]
    TUI[tui
    Ratatui]
  end
  subgraph Core
    Core[codex core
    ConversationManager
    EventMsg]
    Tools[Exec / ApplyPatch
    Browser tools
    Agents]
  end
  subgraph Bridges
    AppSrv[app-server
    MCP JSON-RPC]
  end
  Browser[BrowserManager
  CDP/headless]
  Agents[Agents via core::agent_tool]
  Auto[Auto Drive
  coordinator/controller]

  CLI -->|无子命令| TUI
  CLI -->|exec / auto| Core
  TUI -->|Op (SQ)| Core
  Core -->|EventMsg (EQ)| TUI
  Core --> Tools
  Tools --> Browser
  Tools --> Agents
  TUI -->|审批/键盘| Core
  Core -->|browser_* events| TUI
  Core <--> AppSrv
  AppSrv <-->|MCP JSON-RPC| External
  CLI -->|auto flag| Auto
  Auto <--> Core
  Auto --> Agents
```

### 事件与排序
- 用户输入 → `Op::UserInput`；核心回 `EventMsg::{Answer,Reasoning,ExecCommandBegin/End,PatchApply*,Browser*}`。
- 每条流式事件必须带 `(request_ordinal, output_index, sequence_number)` 与非空 stream id；缺失将被 TUI 丢弃。
- 历史单元由 `history_cell::*` 渲染，`chatwidget` 维护全局排序键。

### Auto Drive 决策 Schema（实际约束）
- 模型必须返回 JSON，字段固定且不可额外扩展：`finish_status`（必填，枚举 `continue|finish_success|finish_failed`）、`status_title`（2-80 字，允许 null）、`status_sent_to_user`（4-600 字，允许 null）、`prompt_sent_to_cli`（必填，最少 4 字符，>600 会被拒绝并提示重试）。
- `agents`（可选/默认启用）：对象，字段 `timing`（`parallel|blocking`）与 `list`（数组，默认最多 5 个项；每项要求 `prompt` 8-400 字、`context` 可空≤1500、`write` bool、`models` 数组可枚举候选模型）。无有效 git worktree 时自动降级为只读代理并在提示中标注。
- `goal` 字段默认不包含，只有在 `AutoDriveSettings.include_goal_field` 启用时才要求，用于无用户提示直接启动场景。
- Schema 验证失败会触发带 Schema 文本的重试日志；连续失败会将错误回传给协调器并中止本轮。

### 浏览器 / CDP
- 核心 `handle_browser_*` 调 `code_browser::BrowserManager`（内置或外部 Chrome）。
- 截图/动作以 `EventMsg::Browser*` 回 TUI，`history_cell/browser.rs` 负责呈现。
- 连接优先级：若配置了 `connect_ws` 或 `connect_port`，仅尝试外部 CDP 连接（无内部回退）；否则启动内置 Chrome，`headless` 开关决定是否有界面。外部连接跳过 viewport 人性化设置并禁用空闲回收；内置浏览器遵循 idle_timeout（默认 24h）并可清理 profile。

### Agents / 子智能体
- `core::agent_tool` 注册与调度；模型白名单校验，写操作可请求独立 worktree；无 git 时自动降级只读。
- 非只读代理若缺少 git 或不在 git 仓库，会直接失败并提示 “Git is required for non-read-only agents”。只读模式下追加提示 `[Running in read-only mode - no modifications allowed]` 并继续执行。
- Auto Drive 可在决策中携带 agent 批次（并行/阻塞）。

### Auto Drive
- `code-auto-drive-core::auto_coordinator` 通过 JSON Schema 约束模型决策：状态文案、`prompt_sent_to_cli`、agent 批次、目标更新。
- `controller` 管理阶段（launch/active/review/pause/backoff/manual）与倒计时，产出 Effect（SubmitPrompt、CancelCoordinator、ResetHistory、SetTaskRunning）。
- `exec` 路径将决策拼成提示，等待核心执行，再把转录 `UpdateConversation` 回协调器。

#### Auto Drive 增强功能（实验性）
`code-auto-drive-core` 包含以下增强组件，通过 `EnhancedCoordinator` 集成：

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Auto Drive Enhanced                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐          │
│  │  Checkpoint      │  │  Diagnostics     │  │  Budget          │          │
│  │  Manager         │  │  Engine          │  │  Controller      │          │
│  └────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘          │
│           └─────────────────────┼─────────────────────┘                     │
│                    EnhancedCoordinator                                      │
│           ┌─────────────────────┼─────────────────────┐                     │
│  ┌────────┴─────────┐  ┌────────┴─────────┐  ┌────────┴─────────┐          │
│  │  Telemetry       │  │  Audit           │  │  Agent           │          │
│  │  Collector       │  │  Logger          │  │  Scheduler       │          │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘          │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **CheckpointManager** (`checkpoint.rs`)：会话持久化与恢复，支持原子保存、校验和验证、过期清理。
- **DiagnosticsEngine** (`diagnostics.rs`)：循环检测（连续相同工具调用）、目标偏离检测、token 异常告警。
- **BudgetController** (`budget.rs`)：token 预算跟踪、轮次/时长限制、80%/100% 阈值告警。
- **AgentScheduler** (`scheduler.rs`)：并行/阻塞智能体调度、并发限制、结果排序聚合。
- **AuditLogger** (`audit.rs`)：操作审计日志、路径/网络权限验证、会话摘要生成。
- **TelemetryCollector** (`telemetry.rs`)：会话/轮次 span 管理、指标收集、错误记录。
- **CompactionEngine** (`compaction.rs`)：语义感知历史压缩、目标保留、token 节省统计。
- **InterventionHandler** (`intervention.rs`)：用户干预状态管理、暂停/恢复/跳过/目标修改。
- **RetryStrategy** (`retry_enhanced.rs`)：错误分类、指数退避、失败计数器管理。

配置通过 `[auto_drive]` 节启用：`checkpoint_enabled`、`diagnostics_enabled`、`token_budget`、`turn_limit`、`audit_enabled` 等。

#### 高吞吐多智能体（SessionPool / TaskPipeline）
- **SessionPool** (`session_pool.rs`)：min=5 / max=20 预热、`scale_up/down_threshold=0.8/0.3` 自扩缩，慢/卡死检测生成 `SessionSlow/SessionStuck/SessionMigrated` 诊断事件并写审计；队列接近 `max_sessions*10` 发 `BackpressureWarning/Exceeded`。
- **ParallelExecution** (`parallel_execution.rs`)：Semaphore 限制 `max_concurrent_agents`（默认 8），低于 8 写入低并发告警；按角色前缀组装提示并合并结果。
- **TaskPipeline + RoleChannel**：阶段定义驱动角色任务，WorkComplete/ErrorOccurred/Guidance/StageAdvance 消息推进阶段，失败会中断流水线。
- **外部记忆与进度日志**：`ai/feature_list.json`（foreman 兼容字段 id/description/module/priority/status/acceptance/testRequirements/tags/version/tddMode/verification），`ai/progress.log` 行格式 `timestamp | type | status | tests | summary | note`，EnhancedCoordinator 在 STEP/CHANGE/VERIFY/REPLAN 时追加。
- **选择性测试/TDD**：`selective_tests.rs` 基于 `git diff` + backlog 生成测试计划，strict 模式缺测或失败会生成带 reason 的 VerificationResult 并通过 `evaluate_and_record_verification` 写回 backlog/进度日志。更多细节见 `docs/architecture/high_throughput.md`。

### MCP / app-server
- `app-server` 作为 stdin/stdout JSON-RPC 网关，使用 `protocol::mcp_protocol` 类型。
- `code_message_processor` 将 `newConversation` / `sendUserTurn` 等映射到核心 `Op`，监听事件再回推 JSON-RPC 通知。
- Exec/Patch 审批：MCP 端请求 `execCommandApproval` / `applyPatchApproval`，300s 超时默认拒绝。
- 审批超时常量 `APPROVAL_TIMEOUT = 300s`，当前不可配置；超时或反序列化失败一律视为拒绝并告警。

## 关键扩展点
- **新增工具/事件**：在 core 产出新的 `EventMsg`，同时在 `history_cell` 增渲染、`chatwidget` 加顺序处理。
- **浏览器能力**：扩展 `BrowserManager` 或新增 `browser_*` 工具时，保持事件与 UI 状态同步。
- **Agents**：更新 `agent_tool` 规则时同步模型白名单与写权限降级逻辑。
- **Auto Drive**：调整决策 Schema / 阶段需同时更新协调器校验与 TUI auto drive cards 呈现。
- **MCP**：添加 RPC 方法需在 `protocol::mcp_protocol`、`app-server` 收发、核心 `Op`/`EventMsg` 三处对齐。

## 常见坑与警戒
- **流式排序缺键**：缺序列键或 stream id → 事件被 UI 丢弃，历史乱序。
- **Esc/审批优先级**：Esc 语义集中在 `chatwidget`（`auto_should_handle_global_esc` + `handle_key_event`）；审批面板必须冒泡 Esc。
- **sandbox/git 依赖**：写操作/ApplyPatch 需 git 工作树；非 git 环境自动降级只读，否则操作会失败。
- **编译警告即失败**：`./build-fast.sh` 必须零 warning；禁止运行 rustfmt。
- **code-rs 依赖隔离**：不得从 code-rs 以相对路径引用 `../codex-rs`（脚本有守卫）。
- **Telemetry**：默认 exporter `None`、环境 `dev`、`log_user_prompt=false`。启用 OTLP 后仅导出 `code_otel` 目标，字段统一脱敏/截断（默认 800 字符，关闭 `log_user_prompt` 时统一 `[REDACTED]`）；目前仅支持 endpoint/headers/protocol 配置，不支持内嵌 TLS/mTLS，证书处理需外部代理。事件含模型 slug、会话 ID、审批/沙箱策略、token 用量；禁用时不会记录用户提示文本。

## 查看或运行
- 开发主流程：`./build-fast.sh`（默认构建 code-rs）。
- 无头执行：`code exec ...`；自动化：`code exec --auto`（或 `auto` 子命令）。
- MCP 网关：`code app-server`（stdin/stdout JSON-RPC）。
