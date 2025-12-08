# 开发环境测试清单（供本地 AI 执行）

目标：在不全局安装的情况下，用开发构建 `./bin/code` 验证核心功能。所有命令均默认在仓库根目录运行。

## 准备
- 确认 Node.js 20+、Rust 工具链已装好。
- 构建：`npm install && ./build-fast.sh`（已完成可跳过）。
- 使用的二进制：`./bin/code`（或 `.code/working/_target-cache/.../code-rs/dev-fast/code`）。
- 认证：运行时使用 ChatGPT 登录，或提前 `export OPENAI_API_KEY=你的key`。

## 基础自检
1) 版本/帮助  
   - `./bin/code --version`（显示版本即可）  
   - `./bin/code --help`（打印子命令列表）
2) 只读无头烟测  
   - `./bin/code exec "pwd"`（预期返回当前工作目录，不改文件）
3) TUI 只读启动  
   - `./bin/code --read-only "explain this repo structure"`  
   - 预期进入 TUI，输出仓库结构说明，不请求写入。

## 多智能体与编排
4) `/plan` 多智能体共识  
   - 在 TUI 输入：`/plan "summarize main components"`  
   - 预期：提示并行/共识调用，返回简要方案，无修改。
5) `/solve` 只读诊断  
   - `./bin/code --read-only "list key TODOs"`  
   - 预期：列出 TODO/改进点，不执行写入命令。
6) `/code` 写入路径提示  
   - 在 TUI 输入：`/code "add a sample README section (do not apply)"`  
   - 当出现命令或写入请求时，拒绝或终止（保持环境干净），确认审批流程正常。

## Auto Drive（默认只读）
7) 无头 Auto Drive 烟测  
   - `./bin/code exec --auto "list files"`  
   - 预期：自动规划并读取文件，不修改。若需写入/联网再加 `--full-auto`，仅在同意的环境下执行。

## 浏览器与路径选择
8) 文件搜索  
   - 在 TUI 输入框键入 `@`，输入文件前缀，确认可选中并插入路径。
9) 浏览器指令（可选）  
   - 如果有可用 CDP 端口：`/chrome 9222`；或内置：`/browser https://example.com`  
   - 预期：连接成功并回显状态（不要求截图）。

## 配置与沙箱
10) 沙箱/审批切换  
    - 使用：`./bin/code --sandbox read-only --ask-for-approval on-request "check git status"`  
    - 预期：命令受只读限制，执行前提示审批；查看 UI 是否给出审批提示。

## 退出与恢复
11) 退出  
    - 从 TUI 正常退出（Ctrl+C 或 UI 提示），确保无挂起进程。  
12) 可选恢复测试  
    - 若前一步产生会话，尝试 `./bin/code resume --last` 验证恢复入口可用（若无会话则可跳过）。

## 记录结果（示例格式）
- 命令：`./bin/code exec "pwd"` → 结果：成功，输出 `/Users/.../code`
- 命令：`/plan "summarize main components"` → 结果：成功，未写入，返回组件摘要
- Auto Drive：`./bin/code exec --auto "list files"` → 结果：成功，未写入

如遇失败，记录：命令、错误日志片段、是否登录/认证、是否开启代理或沙箱、复现步骤。
