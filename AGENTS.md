# Rust/codex-rs

`codex-rs` 目录存放 Rust 代码：

- crate 名统一以 `codex-` 为前缀，例如 `core` 目录里的 crate 叫 `codex-core`。
- 能在 `format!` 里直接内插变量时，直接用 `{}`。
- 将 `codex-rs` 视为 `openai/codex:main` 的只读镜像；实际修改请在 `code-rs` 下进行。

完成度/构建要求

- 唯一必跑的检查是仓库根目录的 `./build-fast.sh`，必须干净通过。
- 冷缓存运行 `./build-fast.sh` 可能 20+ 分钟，请用足够长的超时等待其完成。
- 视任何编译 warning 为失败，全部修掉（如改名为 `_`、去掉多余的 `mut`、删死代码）。
- 未被要求时不要额外跑 fmt/lint/test（如 `just fmt`、`just fix`、`cargo test`）。
- ***禁止运行 rustfmt***
- 推到 `main` 前先跑 `./pre-release.sh`，对齐发布前的预检（dev-fast 构建、CLI 冒烟、workspace nextest）。

可选回归检查（改动 Rust workspace 推荐跑）

- `cargo nextest run --no-fail-fast`：启用 TUI helper 的全量测试；resume/git-init 兼容后应全绿，旧 Git 可能打印 `--initial-branch` 回退警告但不影响通过。
- 快速聚焦：`cargo test -p code-tui --features test-helpers`、`cargo test -p code-cloud-tasks --tests`、`cargo test -p mcp-types --tests`。

调试回归/缺陷时，先写能失败的测试或复现脚本并确认它会红，再动代码——测不红无法证明修复有效。

## 文档规范

- 保持文档简洁、清晰、最新；删除陈旧内容，不要堆叠免责声明。
- 少讲废话，以简明指引为主。
- 不记录次要/非核心功能；聚焦关键流程与期望。
- 不要提交临时“工作”文档、计划或草稿。

## TUI 历史严格顺序

TUI 对流式内容按轮次严格排序。每条流式插入（Answer 或 Reasoning）都必须带模型提供的稳定键 `(request_ordinal, output_index, sequence_number)`。

- 流式插入必须带非空 stream id。UI 会基于事件的 `OrderMeta` 在插入前播种 `(kind, id)` 的排序键。
- 没有 stream id 的流式内容不会被插入，开发期会以错误日志暴露。

## 提交信息

- 每次提交前先检查暂存区：`git --no-pager diff --staged --stat`（必要时再看 `git --no-pager diff --staged`）。
- 主题需描述改了什么、为何改，避免 “chore: commit local work” 这类占位符。
- 优先用带可选 scope 的 Conventional Commit：如 `feat(tui/history): …`、`fix(core/exec): …`、`docs(agents): …`。
- 主题 ≤ 72 字符；若背景有帮助，可写简短 body。
- 语气用祈使/现在时：“add”“fix”“update”，不要用过去式。
- 合并提交不要自造前缀（如 `merge(main<-origin/main):`），用清晰主题描述：`Merge origin/main: <what changed and how conflicts were resolved>`。

示例：

- `feat(tui/history): show exit code and duration for Exec cells`
- `fix(core/codex): handle SIGINT in on_exec_command_begin to avoid orphaned child`
- `docs(agents): clarify commit-message expectations`

## 如何推送

### 合并式推送策略（不要 rebase）

当需要 “push” 本地工作时：

- 不要 rebase；不要用 `git pull --rebase` 或重放提交。
- 优先把 `origin/main` 合并到当前分支，保留本地历史。
- 若远端只改了发布元数据（如 `codex-cli/package.json` 版本号），默认保留本地改动，仅对这些文件采用远端版本，除非用户另有要求。
- 若有疑虑或冲突涉及非平凡区域，先停下询问。

合并流程（无自动提交）：

- 先提交本地改动：
  - 检查 diff：`git --no-pager diff --stat` 和 `git --no-pager diff`
  - 暂存并提交：`git add -A && git commit -m "<descriptive message of local changes>"`
- 拉取远端：`git fetch origin`
- 合并但不提交：`git merge --no-ff --no-commit origin/main`（停在可手动选择阶段）
- 取舍策略：
  - 默认用 ours：`git checkout --ours .`
  - 版本/包文件如 `codex-cli/package.json` 这类可用 theirs：`git checkout --theirs codex-cli/package.json`
- 暂存并提交合并，如：
  - `git add -A && git commit -m "Merge origin/main: adopt remote version bumps; keep ours elsewhere (<areas>)"`
- 跑 `./build-fast.sh` 后再 `git push`

## 命令执行架构

Codex 的命令执行是事件驱动的：

1. **Core 层**（`codex-core/src/codex.rs`）：
   - `on_exec_command_begin()` 发起命令执行
   - 创建包含命令详情的 `EventMsg::ExecCommandBegin`

2. **TUI 层**（`codex-tui/src/chatwidget.rs`）：
   - `handle_codex_event()` 处理执行事件
   - 管理活跃命令的 `RunningCommand` 状态
   - 创建用于 UI 渲染的 `HistoryCell::Exec`

3. **History Cell**（`codex-tui/src/history_cell.rs`）：
   - `new_active_exec_command()` 创建运行中命令的 cell
   - `new_completed_exec_command()` 在结束时更新
   - 通过 `ParsedCommand` 处理语法高亮

该架构将执行逻辑（core）、UI 状态（chatwidget）与渲染（history_cell）分离。

### Auto Drive 的 Esc 处理

- Auto Drive 的 Esc 分发都在 `code-rs/tui/src/chatwidget.rs`：`ChatWidget::auto_should_handle_global_esc` 决定全局 Esc 是否让位给 Auto Drive，`ChatWidget::handle_key_event` 负责停止/暂停。调整 Esc 语义时两个地方一起改。
- 审批面板绝不拦截 Esc；`code-rs/tui/src/bottom_pane/auto_coordinator_view.rs` 会让 Esc（和其他审批快捷键）冒泡回聊天窗口，改视图层时保持此约定。
- 避免在其他地方新增 Auto Drive 的 Esc 处理，否则会打乱 `app.rs` 的模态优先级，导致无法可靠停止运行。

## 编写新的 UI 回归测试

- 从 `make_chatwidget_manual()`（或 `make_chatwidget_manual_with_sender()`）开始，构建带内存通道的 `ChatWidget`。
- 用小枚举（如 `ScriptStep`）喂 `chat.handle_key_event()` 模拟输入；`tests.rs` 里的 `run_script()` 提供现成 helper 并驱动 `AppEvent`。
- 交互后用 `ratatui::Terminal`/`TestBackend` 渲染，再用 `buffer_to_string()`（包装 `strip_ansi_escapes`）规范化 ANSI 输出再断言。
- 优先用快照断言（`assert_snapshot!`）或富字符串对比，方便发现回归。保持确定性：修剪行尾空格，按现有测试的节奏推进 commit tick。
- 新增或更新快照时，用显式开关（如 `UPDATE_IDEAL=1`）控制重写。

## VT100 快照工具

- VT100 工具位于 `code-rs/tui/tests/vt100_chatwidget_snapshot.rs`，把 live `ChatWidget` 渲染到 `Terminal<VT100Backend>`，捕获用户在 PTY 看到的完整输出（含框架、输入行、流式插入）。
- 使用 `code_tui::test_helpers` 的 `ChatWidgetHarness` 预置历史/事件并消费 `AppEvent`。单帧用 `render_chat_widget_to_vt100(width, height)`，多帧流式用 `render_chat_widget_frames_to_vt100(&[(w,h), ...])`。
- 工具暴露 `layout_metrics()`，可断言滚动偏移与视口高度，无需访问私有字段。
- 快照确定性：测试自动设 `CODEX_TUI_FAKE_HOUR=12` 避免问候语波动；如需其他时间，构造前重写 env。
- 新场景：推送历史/事件，再调用渲染；可用 `insta::assert_snapshot!` 或手动断言字符串。多帧时先推事件再按 UI 顺序捕获帧。
- 跑全部 VT100 快照：
  - `cargo test -p code-tui --test vt100_chatwidget_snapshot --features test-helpers -- --nocapture`
- 有意改渲染时，查看 `code-rs/tui/tests/snapshots/` 下出现的 `.snap.new`，用 `cargo insta review` / `cargo insta accept`（尽量仅限此测试）接受。

### 监控发布流水线

- 用 `scripts/wait-for-gh-run.sh` 跟踪 GitHub Actions 的 release，避免手动反复跑 `gh`。
- 推送后常用：`scripts/wait-for-gh-run.sh --workflow Release --branch main`。
- 已知 run id 时：`scripts/wait-for-gh-run.sh --run <run-id>`。
- 通过 `--interval <seconds>` 调整轮询频率（默认 8）。成功退出码 0，失败 1，可用于本地自动化。
- 加 `--failure-logs` 自动 dump 失败任务的日志。
- 依赖：PATH 里需要 GitHub CLI（`gh`）和 `jq`。
