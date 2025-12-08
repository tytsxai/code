# Every CODE

&ensp;

<img src="docs/images/every-logo.png" alt="Every Code Logo" width="400">

&ensp;

**Every Code**（简称 Code）是一款运行在终端里的本地快速编程智能体。它是社区驱动的 `openai/codex` 分支，专注真实的开发体验：浏览器集成、多智能体、主题与推理控制，同时与上游保持兼容。

&ensp;
## v0.5.0 有哪些更新（2025 年 11 月 21 日）

- **更名为 Every Code**——便于被发现，仍保留 `code` 这个简写。
- **Auto Drive 升级**——给 `/auto` 一个任务，它会自行规划、协调智能体、重跑检查并在异常时恢复，无需人工看护。
- **大量易用性改进**——`/resume` 和 `/undo` 可靠运行，移植了所有主要的上游特性，包括 compaction v2 与 -max/-mini 模型。
- **统一设置中心**——`/settings` 集中管理限额、模型路由、主题和 CLI 集成，一处即可审计配置。
- **卡片式活动视图**——智能体、浏览器会话、网络搜索和 Auto Drive 以卡片呈现，可展开查看完整日志。
- **性能加速**——历史渲染与流式展示经过优化，即便长时间多智能体会话也保持流畅。
- **更聪明的智能体**——可为 `/plan`、`/code`、`/solve` 按需选择编排 CLI（Claude、Gemini、GPT-5、Qwen 等）。

完整变更见 `docs/release-notes/RELEASE_NOTES.md`。

&ensp;
## 为什么选择 Every Code

- 🚀 **Auto Drive 编排**——多智能体自动化，能自愈并交付完整任务。
- 🌐 **浏览器集成**——CDP 支持、无头浏览、截图内嵌。
- 🤖 **多智能体命令**——`/plan`、`/code`、`/solve` 协同多个 CLI 智能体。
- 🧭 **统一设置中心**——`/settings` 覆盖限额、主题、审批与提供商接入。
- 🎨 **主题系统**——可切换无障碍主题、定制强调色、通过 `/themes` 即时预览。
- 🔌 **MCP 支持**——可扩展文件系统、数据库、API 或自定义工具。
- 🔒 **安全模式**——只读、审批与工作区沙箱。

&ensp;
## AI 视频

&ensp;
<p align="center">
  <a href="https://youtu.be/UOASHZPruQk">
    <img src="docs/images/video-auto-drive-new-play.jpg" alt="播放 Auto Drive 介绍视频" width="100%">
  </a><br>
  <strong>Auto Drive 概览</strong>
</p>

&ensp;
<p align="center">
  <a href="https://youtu.be/sV317OhiysQ">
    <img src="docs/images/video-v03-play.jpg" alt="播放多智能体宣传视频" width="100%">
  </a><br>
  <strong>多智能体演示</strong>
</p>


&ensp;
## 快速开始

### 直接运行

```bash
npx -y @just-every/code
```

### 安装并运行

```bash
npm install -g @just-every/code
code // 如果已被 VS Code 占用可用 `coder`
```

注意：若已有 `code` 命令（如 VS Code），CLI 也会安装 `coder`。冲突时使用 `coder`。

**认证方式**（三选一）：
- **ChatGPT 登录**（Plus/Pro/Team；使用你计划可用的模型）
  - 运行 `code` 选择 "Sign in with ChatGPT"
- **API Key**（按量计费）
  - 设置 `export OPENAI_API_KEY=xyz` 然后运行 `code`
- **第三方激活器**（如 codex-activator）
  - 本项目特别支持通过激活器使用代理服务，详见下方「激活器集成」章节

### 安装 Claude 与 Gemini（可选）

Every Code 支持编排其他 AI CLI。安装它们并配置后即可与 Code 一起使用。

```bash
# 确保本地有 Node.js 20+（安装到 ~/.n）
npm install -g n
export N_PREFIX="$HOME/.n"
export PATH="$N_PREFIX/bin:$PATH"
n 20.18.1

# 安装配套 CLI
export npm_config_prefix="${npm_config_prefix:-$HOME/.npm-global}"
mkdir -p "$npm_config_prefix/bin"
export PATH="$npm_config_prefix/bin:$PATH"
npm install -g @anthropic-ai/claude-code @google/gemini-cli @qwen-code/qwen-code

# 快速自检
claude --version
gemini --version
qwen --version
```

> ℹ️ 将 `export N_PREFIX="$HOME/.n"` 与 `export PATH="$N_PREFIX/bin:$PATH"`（加上 `npm_config_prefix` 的 bin 路径）写入 shell 配置，以便下次会话仍可访问这些 CLI。

&ensp;
## 命令

### 浏览器
```bash
# 连接外部 Chrome（CDP）
/chrome        # 自动检测端口连接
/chrome 9222   # 指定端口连接

# 切换到内置浏览器模式
/browser       # 使用内置无头浏览器
/browser https://example.com  # 在内置浏览器中打开 URL
```

### Agents
```bash
# 规划改动（Claude、Gemini、GPT-5 共识）
# 所有智能体审阅任务并创建合并计划
/plan "Stop the AI from ordering pizza at 3AM"

# 解决复杂问题（Claude、Gemini、GPT-5 竞速）
# 最快的优先（参见 https://arxiv.org/abs/2505.17813）
/solve "Why does deleting one user drop the whole database?"

# 写代码！（Claude、Gemini、GPT-5 共识）
# 创建多个工作树并实施最优方案
/code "Show dark mode when I feel cranky"
```

### Auto Drive
```bash
# 交给多步骤自动化；Auto Drive 会协调智能体和审批
/auto "Refactor the auth flow and add device login"

# 恢复或查看进行中的 Auto Drive
/auto status
```

### 通用
```bash
# 试用新主题
/themes

# 调整推理力度
/reasoning low|medium|high

# 切换模型或努力档
/model

# 开启新对话
/new
```

## CLI 参考

```shell
code [options] [prompt]

Options:
  --model <name>        覆盖模型（gpt-5.1、claude-opus 等）
  --read-only          阻止文件修改
  --no-approval        跳过审批提示（谨慎使用）
  --config <key=val>   覆盖配置项
  --oss                使用本地开源模型
  --sandbox <mode>     设置沙箱级别（read-only、workspace-write 等）
  --help              显示帮助
  --debug             将 API 请求/响应写入日志
  --version           显示版本号
```

&ensp;
## 记忆与项目文档

Every Code 可在会话间记忆上下文：

1. **在项目根创建 `AGENTS.md` 或 `CLAUDE.md`**：
```markdown
# Project Context
This is a React TypeScript application with:
- Authentication via JWT
- PostgreSQL database
- Express.js backend

## Key files:
- `/src/auth/` - Authentication logic
- `/src/api/` - API client code  
- `/server/` - Backend services
```

2. **会话记忆**：保留对话历史
3. **代码库分析**：自动理解项目结构

&ensp;
## 非交互 / CI 模式

适用于自动化与 CI/CD：

```shell
# 运行特定任务
code --no-approval "run tests and fix any failures"

# 生成报告
code --read-only "analyze code quality and generate report"

# 批处理
code --config output_format=json "list all TODO comments"
```

&ensp;
## Model Context Protocol (MCP)

Every Code 支持 MCP 扩展能力：

- **文件操作**：高级文件系统访问
- **数据库连接**：查询与修改数据库
- **API 集成**：连接外部服务
- **自定义工具**：构建自定义扩展

在 `~/.code/config.toml` 配置 MCP。为每个服务创建命名表，例如 `[mcp_servers.<name>]`（与其他客户端使用的 `mcpServers` JSON 对象对应）：

```toml
[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/project"]
```

&ensp;
## 配置

主配置文件：`~/.code/config.toml`

> [!NOTE]
> Every Code 同时读取 `~/.code/` 与 `~/.codex/`（兼容旧版），但只会写入 `~/.code/`。若切回 Codex 启动失败，删除 `~/.codex/config.toml`。若升级后缺少设置，可将旧的 `~/.codex/config.toml` 复制到 `~/.code/`。

```toml
# Model settings
model = "gpt-5.1"
model_provider = "openai"

# Behavior
approval_policy = "on-request"  # untrusted | on-failure | on-request | never
model_reasoning_effort = "medium" # low | medium | high
sandbox_mode = "workspace-write"

# UI preferences see THEME_CONFIG.md
[tui.theme]
name = "light-photon"

# Add config for specific models
[profiles.gpt-5]
model = "gpt-5.1"
model_provider = "openai"
approval_policy = "never"
model_reasoning_effort = "high"
model_reasoning_summary = "detailed"
```

### 环境变量

- `CODE_HOME`：自定义配置目录位置
- `OPENAI_API_KEY`：使用 API Key 而非 ChatGPT 登录
- `OPENAI_BASE_URL`：使用备用 API 端点
- `OPENAI_WIRE_API`：强制内置 OpenAI 提供商使用 `chat` 或 `responses` 接口

&ensp;
## 激活器集成

本项目特别支持第三方激活器（如 `codex-activator`），可实现一次激活即在多个版本间共享认证。

### 工作原理

激活器将配置写入 `~/.codex/config.toml`，包含自定义 `model_provider` 和环境变量。本项目通过同步该配置来复用激活器的认证。

### 快速配置

```bash
# 1. 安装并运行激活器
codex-activator

# 2. 同步配置（添加到 ~/.zshrc 实现自动同步）
if [ -f ~/.codex/config.toml ]; then
  cp ~/.codex/config.toml ~/.code/config.toml 2>/dev/null
fi

# 3. 重新加载环境
source ~/.zshrc

# 4. 验证
code exec "echo hello"
```

### 更换激活码

1. 运行 `codex-activator` 输入新激活码
2. 打开新终端（配置自动同步）
3. 直接使用 `code`

详细说明请参阅 [docs/activator-integration.md](docs/activator-integration.md)。

&ensp;
## FAQ

**与原版有何不同？**
> 本分支增加了浏览器集成、多智能体命令（`/plan`、`/solve`、`/code`）、主题系统与推理控制，并保持完全兼容。

**可以复用现有的 Codex 配置吗？**
> 可以。Every Code 会同时读取 `~/.code/`（主目录）与旧版 `~/.codex/`。只写入 `~/.code/`，切回 Codex 仍可运行；如发现冲突，可复制或删除旧文件。

**能配合 ChatGPT Plus 吗？**
> 完全可以。沿用原有的 “Sign in with ChatGPT” 流程。

**数据安全吗？**
> 安全。认证留在本机，我们不会代理你的凭据或对话。

&ensp;
## 贡献

欢迎贡献！Every Code 在保持与上游兼容的同时加入社区需求的功能。

### 开发流程

```bash
# 克隆与安装依赖
git clone https://github.com/just-every/code.git
cd code
npm install

# 构建（开发时使用快速构建）
./build-fast.sh

# 本地运行
./code-rs/target/dev-fast/code
```

### 提交 Pull Request

1. Fork 仓库
2. 创建特性分支：`git checkout -b feature/amazing-feature`
3. 实施改动
4. 运行测试：`cargo test`
5. 确认构建通过：`./build-fast.sh`
6. 提交 PR


&ensp;
## 法律与使用

### 许可证与归属
- 本项目是 `openai/codex` 的社区分支，沿用 **Apache-2.0** 许可证并保留上游 LICENSE 与 NOTICE。
- **Every Code**（Code）**并非** OpenAI 关联或认可。

### 你的责任
通过 Every Code 使用 OpenAI、Anthropic 或 Google 服务即表示你同意**它们的条款与政策**。尤其：
- **不要** 在非预期路径下抓取/提取内容。
- **不要** 绕过或干扰限流、配额或安全措施。
- 使用你**自己的**账号；不要共享或轮换账号以逃避限制。
- 若配置其他模型提供商，你需遵守相应条款。

### 隐私
- 认证文件位于 `~/.code/auth.json`
- 你发送给模型的输入/输出遵循各提供商条款与隐私政策；请查看这些文档（以及组织级数据共享设置）。

### 可能变更
AI 提供商可能调整资格、限额、模型或认证流程。Every Code 同时支持 ChatGPT 登录与 API Key 模式，可按需选择（本地/爱好 vs CI/自动化）。

&ensp;
## 许可证

Apache 2.0 - 详见 [LICENSE](LICENSE)。

Every Code 是原始 Codex CLI 的社区分支，在保持兼容的同时提供开发者社区期待的增强功能。

&ensp;
---
**需要帮助？** 在 [GitHub](https://github.com/just-every/code/issues) 提交 issue 或查看我们的文档。
