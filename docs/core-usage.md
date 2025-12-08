# 核心上手指南

面向初次使用 Every Code 的中文用户：快速完成安装、认证，并理解核心用法。

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

## 常用启动方式
- 交互式 TUI：`code` 或 `code "fix lint errors"`。
- 非交互/脚本：`code exec "run tests"`（默认只读）；允许修改与联网用 `code exec --full-auto ...`。
- 全自动编排：`/auto "<目标>"`（TUI）；或 `code exec --auto "<目标>"`。TUI 模式需配置 `sandbox_mode=danger-full-access` 且 `approval_policy=never` 才能无人值守。

## 多智能体与配置
- 默认内置多家 CLI；多智能体命令 `/plan` `/code` `/solve` 会自动并行/共识调用。
- 自定义/启用特定 CLI：编辑 `~/.code/config.toml`（兼容读取 `~/.codex/config.toml`），添加 `[[agents]]`：
  ```toml
  [[agents]]
  name = "claude-opus-4.5"
  command = "claude"
  read_only = false
  enabled = true
  ```
- 仅想让子智能体只读，可设 `read_only=true` 或在会话中使用 `--sandbox read-only`。

## 关键路径与提示
- 配置：`~/.code/config.toml`；项目/个人指令：`AGENTS.md`（支持 `~/.code/`、仓库根、子目录）。
- 自定义提示：在 `~/.code/prompts/` 放 `.md`，输入 `/文件名` 可直接引用。
- 浏览器：`/chrome` 连接外部 CDP；`/browser` 用内置无头浏览器。
- 快捷安全切换：`--sandbox read-only|workspace-write|danger-full-access`，`--ask-for-approval <策略>`。

## 快速自检
- 查看版本：`code --version`
- 验证 CLI 工作：`code --read-only "explain this repo structure"` 或 `code exec "pwd"`。
