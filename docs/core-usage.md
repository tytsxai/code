# 核心上手指南

面向初次使用 Every Code 的中文用户：用最少步骤完成安装、登录、首轮尝试和自动化。

## 环境与安装
- 依赖：Node.js 20+；从源码构建需 Rust 工具链（`./build-fast.sh` 会自动使用 rustup）。
- 试用（一次性）：`npx -y @just-every/code`
- 全局安装：`npm install -g @just-every/code`；启动命令 `code`（若与 VS Code 冲突则用 `coder`）。
- 源码构建（仓库根执行）：
  ```bash
  npm install
  ./build-fast.sh
  ./bin/code --version        # build-fast 会将产物复制到 bin
  ```

## 认证方式（必选其一）
- ChatGPT 登录：运行 `code`，选择 “Sign in with ChatGPT”。
- API Key：`export OPENAI_API_KEY=sk-...` 后运行 `code` 或 `code exec ...`。

## 首次运行三步
1) 认证：按上节两种方式其一完成登录/API Key。
2) 进入 TUI：`code`。如需初始提示：`code "fix lint errors"`。
3) 运行示例：
   - 只读了解仓库：`code --read-only "explain this repo"`（或在 TUI 输入 `/solve "explain this repo"`）。
   - 修改/执行：在 TUI 输入 `/code "add a healthcheck endpoint"`，按提示审批命令与写入。
   - 自动化脚本：`code exec "run tests"`（默认只读）；允许修改与联网用 `code exec --full-auto "fix tests"`。

## Auto Drive（全自动）
- TUI：`/auto "<目标>"`。需在设置或 `config.toml` 将 `sandbox_mode=danger-full-access`、`approval_policy=never` 才能无人值守。
- CLI：`code exec --auto "<目标>"`（等价于 headless Auto Drive），如需写入与联网加 `--full-auto`。
- 倒计时/手动确认：在 `/auto settings` 选择 continue 模式；Esc 可随时暂停/停止。

## 多智能体与配置
- 默认内置多家 CLI；多智能体命令 `/plan` `/code` `/solve` 自动并行/共识调用。
- 指定/替换 CLI：编辑 `~/.code/config.toml`（兼容读取 `~/.codex/config.toml`），添加 `[[agents]]`：
  ```toml
  [[agents]]
  name = "claude-opus-4.5"
  command = "claude"
  read_only = false
  enabled = true
  ```
- 全局安全策略：启动参数或配置 `sandbox_mode`（read-only/workspace-write/danger-full-access）、`approval_policy`（never/on-request/on-failure/untrusted）。只读需求时用 `--sandbox read-only` 或在配置文件中设定。
- 子智能体强制只读：`read_only=true`。

## 常用路径与快捷键
- 配置：`~/.code/config.toml`；项目/个人指令：`AGENTS.md`（支持 `~/.code/`、仓库根、子目录）。
- 自定义提示：在 `~/.code/prompts/` 放 `.md`，输入 `/文件名` 直接引用；支持 `$1..$9` 位置参数。
- 浏览器：`/chrome` 连接外部 CDP；`/browser` 用内置无头浏览器。
- 文件搜索：在输入框键入 `@` 触发模糊搜文件，Enter/Tab 插入路径。
- Esc×2 回溯编辑上一条消息；`/themes` 预览主题。

## 快速自检
- 查看版本：`code --version`
- CLI 烟测：`code --read-only "explain this repo structure"` 或 `code exec "pwd"`。
- Auto Drive 烟测（只读）：`code exec --auto "list files"`；如需写入请确认沙箱/审批已放开。

## 快速自检
- 查看版本：`code --version`
- 验证 CLI 工作：`code --read-only "explain this repo structure"` 或 `code exec "pwd"`。
