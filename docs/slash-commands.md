# 斜杠命令

Code CLI 支持在输入框开头输入的斜杠命令，用于快捷操作、开关或自动展开为完整提示。本文列出所有内置命令及其作用。

注意

- 命令会显示在 TUI 的斜杠弹窗中，以下顺序与 UI 一致。
- 标注为“prompt‑expanding”的命令会将输入转换成完整提示，通常会触发多智能体流程。
- 部分命令接受参数；如为必填，使用方式会在括号中给出。

## 导航与会话

- `/browser`：打开内置浏览器。
- `/chrome`：连接到你的 Chrome 浏览器。
- `/new`：在对话中开始新聊天。
- `/resume`：恢复此文件夹的过去会话。
- `/quit`：退出 Code。
- `/logout`：登出。
- `/login`：管理 Code 登录（选择、添加或断开账号）。
- `/settings [section]`：打开设置面板。可选 section 直接跳到 `model`、`theme`、`agents`、`auto`、`review`、`validation`、`limits`、`chrome`、`mcp` 或 `notifications`。

## 工作区与 Git

- `/init`：创建包含 Code 指南的 `AGENTS.md`。
- `/diff`：显示 `git diff`（包含未跟踪文件）。
- `/undo`：打开快照选择器，可将工作区文件恢复到某个 Code 快照，并可选回溯对话至该点。
- `/branch [task]`：创建工作树分支并切换。如提供任务/描述，会用于分支命名。必须在仓库根目录运行（不要在其他工作树内）。设置 `CODE_BRANCH_COPY_CACHES=1`（旧版 `CODEX_BRANCH_COPY_CACHES=1`）可将 `node_modules` 和 Rust 构建缓存镜像到工作树；否则不会自动复制缓存目录。
- `/merge`：将当前工作树分支合并回默认分支并删除该工作树。需在 `/branch` 创建的工作树内运行。
- `/push`：让 Code 按受控流程提交、推送并监控工作流。若工作区已干净或缺少必需工具/文件，会自动跳过清理或 GitHub 监控步骤。
- `/review [focus]`：无参数时打开审查选择器，可审计工作区、特定提交、与其他分支对比或输入自定义指令。有 focus 参数时跳过选择器直接使用你的文本。需要自动修复并重复检查时，可在 `/settings review` 配置 Auto Resolve 与最大复审次数（默认为 5）。
- `/cloud`：浏览 Code Cloud 任务、查看详情、应用补丁并在 TUI 中创建新任务。
- `/cmd <name>`：运行当前工作区定义的项目命令。

## 体验与显示

- `/theme`：自定义应用主题。
- `/verbosity (high|medium|low)`：调整文本详尽程度。
- `/model`：选择默认模型。
- `/reasoning (minimal|low|medium|high)`：调整推理力度。
- `/prompts`：显示示例提示。
- `/status`：查看当前会话配置与 token 用量。
- `/limits`：调整会话限制并可视化小时/周限流使用情况。
- `/update`：检查已安装版本、发现可用升级，并在可能时开启交互式安装器的引导升级终端。
- `/notifications [status|on|off]`：管理通知设置。无参数时显示通知面板；参数 `status` 显示当前配置，`on` 全开，`off` 全关。
- `/mcp [status|on|off <name>|add]`：管理 MCP 服务器。无参数时显示所有服务器并可切换；参数 `status` 列出服务器，`on <name>` 开启，`off <name>` 关闭，`add` 启动新服务器配置流程。
- `/validation [status|on|off|<tool> (on|off)]`：查看或切换验证工具设置。

## 搜索与提及

- `/mention`：提及文件（打开文件搜索以快速插入）。

## 性能与 Agents

- `/perf (on|off|show|reset)`：性能追踪控制。
- `/agents`：配置 agents 与子智能体命令（含自动跟进与观察者状态；在 dev、dev-fast、perf 版本可用）。
- `/auto [goal]`：启动维护者风格的自动协调器。若未提供目标，默认值为“review the git log for recent changes and come up with sensible follow up work”。

## Prompt‑Expanding（多智能体）

以下命令会展开成完整提示（由 `code-core` 生成），通常会启动多个智能体。需要提供任务/问题描述。

- `/plan <task>`：创建完整计划（多智能体）。Prompt‑expanding。
- `/solve <problem>`：解决挑战性问题（多智能体）。Prompt‑expanding。
- `/code <task>`：执行编码任务（多智能体）。Prompt‑expanding。

## 仅开发版

- `/demo`：填充聊天历史以各种示例单元（用于 dev 与 perf 构建的 UI 测试）。
- `/test-approval`：测试审批请求（仅 debug 构建）。

实现说明

- 权威命令列表定义在 `code-rs/tui/src/slash_command.rs`（`SlashCommand` 枚举）。添加新命令时请更新本文档以保持 UI 与文档一致。
- `/plan`、`/solve`、`/code` 的提示格式在 `code-rs/core/src/slash_commands.rs`。
  当未配置 `[[agents]]` 时，编排器会向 LLM 宣告以下模型 slug 用于多智能体运行：`code-gpt-5.1`、`claude-sonnet-4.5`、`claude-opus-4.1`、`gemini-3-pro`、`gemini-2.5-pro`、`gemini-2.5-flash`、`qwen-3-coder`、`code-gpt-5.1-codex`、`code-gpt-5.1-codex-mini`（`cloud-gpt-5.1-codex` 由 `CODE_ENABLE_CLOUD_AGENT_MODEL` 控制）。可通过 `[[agents]]` 或按命令的 `[[subagents.commands]].agents` 替换或固定该列表。
