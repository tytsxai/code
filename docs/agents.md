# Agents 与子智能体

Every Code 可以启动外部 CLI “智能体”，并在 `/plan`、`/solve`、`/code` 等多智能体流程中编排它们。

## 智能体配置（`config.toml` 中的 `[[agents]]`）
```toml
[[agents]]
name = "code-gpt-5.1-codex-max"   # 在选择器中显示的 slug/别名
command = "coder"                # 可执行文件；默认等于 name
args = ["--foo", "bar"]          # 基础 argv
args_read_only = ["-s", "read-only", "-a", "never", "exec", "--skip-git-repo-check"]
args_write = ["-s", "workspace-write", "--dangerously-bypass-approvals-and-sandbox", "exec", "--skip-git-repo-check"]
env = { CODE_FOO = "1" }
read_only = false                 # 即便会话允许写入也强制只读
enabled = true                    # 置为 false 时在选择器中隐藏
description = "Frontline coding agent"
instructions = "添加到该智能体提示的前言"
```
字段摘要：`name`（slug/别名）、`command`（可用绝对路径）、`args*`（RO/RW 列表会覆盖基础参数）、`env`、`read_only`、`enabled`，可选 `description` 与 `instructions`。

### 内置默认值
若未配置任何 `[[agents]]`，Code 会内置一组智能体（云端变体受环境变量 `CODE_ENABLE_CLOUD_AGENT_MODEL` 控制）：`code-gpt-5.1-codex-max`、`claude-opus-4.5`、`gemini-3-pro`、`code-gpt-5.1-codex-mini`、`claude-sonnet-4.5`、`gemini-2.5-flash`、`code-gpt-5.1`、`claude-haiku-4.5`、`qwen-3-coder`、`cloud-gpt-5.1-codex`。内置配置会移除用户提供的 `--model/-m` 以避免冲突，并插入自身参数。

## 子智能体（`[[subagents.commands]]`）
```toml
[[subagents.commands]]
name = "plan"                     # 斜杠命令（/plan、/solve、/code 或自定义）
read_only = true                  # plan/solve 默认 true，code 默认 false
agents = ["code-gpt-5.1-codex-max", "claude-opus-4.5"]  # 为空则回退到已启用智能体或内置列表
orchestrator_instructions = "编排器在启动智能体前的指导"
agent_instructions = "附加到每个子智能体提示的前言"
```
- `name`：创建/覆盖的斜杠命令。
- `read_only`：为 true 时强制子智能体只读。
- `agents`：显式列表；为空 → 启用的 `[[agents]]`；若未配置则使用内置 roster。
- `orchestrator_instructions`：在发出 `agent.create` 前附加到 Code 侧提示。
- `agent_instructions`：附加到每个子智能体提示。

编排器会并行多个智能体，等待结果，并根据你的 `hide_agent_reasoning` / `show_raw_agent_reasoning` 设置合并推理。

## TUI 控制
- `/agents` 打开设置覆盖层的 Agents 部分：可切换启用/只读、查看默认值、打开编辑器。
- 智能体编辑器：创建或编辑单个智能体（启用/禁用、只读、instructions）。参数/env 来源于 `config.toml`。
- 子智能体编辑器：配置按命令的智能体列表、只读标记与 instructions。内置 `/plan` `/solve` `/code` 也可同样覆盖。
- 模型选择器是模态窗口，选择后会回到调用的区域。

## Auto Drive 交互
- Auto Drive 使用设置里的 `agents_enabled` 开关；关闭时协调器跳过智能体批次。
- 如果没有 git 仓库，Auto Drive 会指示所有智能体以只读方式运行。
- `AUTO_AGENTS.md` 会与 `AGENTS.md` 一起为 Auto Drive 提供专属指导。

## AGENTS.md 与项目记忆
- Code 会沿路径加载 AGENTS.md（全局、仓库根、当前目录），总大小上限 32 KiB；越靠近当前路径优先级越高。
- 内容会在第一轮作为 system/developer 指令注入；直接的用户/开发者提示仍优先。

## Windows 发现技巧
- 在 Windows 上，`command` 需包含扩展名（`.exe`、`.cmd`、`.bat`、`.com`）。
- NPM 全局路径常在 `C:\\Users\\<you>\\AppData\\Roaming\\npm\\`。
- 若 PATH 不稳定，请在 `[[agents]]` 中使用绝对 `command` 路径。

## 通知与推理可见性
- `hide_agent_reasoning = true` 会在 TUI 与 `code exec` 中隐藏智能体推理流。
- `show_raw_agent_reasoning = true` 会在模型提供时展示原始 chain-of-thought。
- 通知过滤可通过 `/notifications` 或 `config.toml` 中的 `notify` / `tui.notifications` 控制。

## 无头 `code exec`
- `code exec --json` 会流式输出 JSONL 事件（包含智能体轮次）。
- `--output-schema <schema.json>` 强制结构化 JSON 输出；与 `--output-last-message` 组合以仅保存最终载荷。
- `code exec` 默认只读；添加 `--full-auto` 并设置可写沙箱以允许修改。

## 快速示例
- 自定义智能体：
```toml
[[agents]]
name = "my-coder"
command = "/usr/local/bin/coder"
args_write = ["-s", "workspace-write", "--dangerously-bypass-approvals-and-sandbox", "exec", "--skip-git-repo-check"]
enabled = true
```
- 自定义上下文扫描命令：
```toml
[[subagents.commands]]
name = "context"
read_only = true
agents = ["code-gpt-5.1-codex-max", "claude-opus-4.5"]
orchestrator_instructions = "让每个智能体总结最相关的文件和测试。"
agent_instructions = "返回路径并给出 1–2 句理由；不要修改文件。"
```
