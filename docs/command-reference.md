# 命令速查表

本文档整合了 Every Code 的所有命令，方便快速查阅。

## CLI 命令

### 基本用法

```bash
code                          # 启动交互式 TUI
code "任务描述"               # 启动 TUI 并带初始提示
code exec "任务描述"          # 非交互模式执行
```

### 常用参数

| 参数 | 说明 | 示例 |
|------|------|------|
| `-m, --model <name>` | 指定模型 | `code -m gpt-5.1` |
| `-a, --ask-for-approval` | 审批策略 | `code -a never` |
| `--read-only` | 只读模式 | `code --read-only "..."` |
| `--full-auto` | 全自动模式（可写入+联网） | `code exec --full-auto "..."` |
| `--sandbox <mode>` | 沙箱级别 | `code --sandbox danger-full-access` |
| `-i, --image <path>` | 附加图片 | `code -i img.png "解释这个"` |
| `-C, --cd <dir>` | 指定工作目录 | `code -C /path/to/project` |
| `--json` | JSON 输出模式 | `code exec --json "..."` |
| `-o, --output-last-message` | 输出到文件 | `code exec "..." -o result.txt` |

### exec 模式专用

| 命令 | 说明 |
|------|------|
| `code exec "任务"` | 非交互执行（默认只读） |
| `code exec --full-auto "任务"` | 允许文件修改 |
| `code exec resume --last "继续"` | 恢复上次会话 |
| `code exec resume <ID> "继续"` | 恢复指定会话 |

---

## TUI 斜杠命令

### 导航与会话

| 命令 | 说明 |
|------|------|
| `/new` | 开始新对话 |
| `/resume` | 恢复历史会话 |
| `/quit` | 退出 |
| `/login` | 管理登录账号 |
| `/logout` | 登出 |
| `/settings [section]` | 打开设置（可选：model/theme/agents/auto 等） |

### 浏览器

| 命令 | 说明 |
|------|------|
| `/browser` | 打开内置浏览器 |
| `/browser <url>` | 在内置浏览器打开 URL |
| `/chrome` | 连接外部 Chrome（自动检测端口） |
| `/chrome 9222` | 连接指定端口的 Chrome |

### 多智能体命令 ⭐

| 命令 | 说明 |
|------|------|
| `/plan <任务>` | 多智能体协作制定计划 |
| `/solve <问题>` | 多智能体竞速解决问题 |
| `/code <任务>` | 多智能体协作编写代码 |
| `/auto [目标]` | Auto Drive 全自动编排 |

### Git 与工作区

| 命令 | 说明 |
|------|------|
| `/diff` | 显示 git diff |
| `/undo` | 快照回滚 |
| `/branch [描述]` | 创建工作树分支 |
| `/merge` | 合并当前工作树回主分支 |
| `/push` | 提交、推送并监控工作流 |
| `/review [focus]` | 代码审查 |
| `/init` | 创建 AGENTS.md |
| `/compact` | 压缩对话以节省上下文 |

### 模型与显示

| 命令 | 说明 |
|------|------|
| `/model` | 选择模型与推理力度 |
| `/approvals` | 配置审批策略 |
| `/reasoning <level>` | 推理力度（minimal/low/medium/high） |
| `/verbosity <level>` | 输出详尽度（low/medium/high） |
| `/theme` | 自定义主题 |
| `/status` | 查看会话配置与 token 用量 |
| `/limits` | 调整限额 |

### 工具管理

| 命令 | 说明 |
|------|------|
| `/mcp [status\|on\|off <name>\|add]` | 管理 MCP 服务器 |
| `/agents` | 配置 agents |
| `/validation [on\|off]` | 验证工具设置 |
| `/notifications [on\|off]` | 通知设置 |

### 其他

| 命令 | 说明 |
|------|------|
| `/mention` | 提及文件（@ 搜索） |
| `/prompts` | 显示示例提示 |
| `/update` | 检查更新 |
| `/cloud` | 浏览 Cloud 任务 |
| `/cmd <name>` | 运行项目命令 |
| `/perf <on\|off\|show\|reset>` | 性能追踪 |
| `/feedback` | 发送日志给维护者 |

---

## 快捷键

| 快捷键 | 说明 |
|--------|------|
| `@` | 模糊搜索文件 |
| `Ctrl+V / Cmd+V` | 粘贴图片 |
| `Esc` → `Esc` | 编辑上一条消息 |
| `Esc` | 在预览中暂停/返回 |

---

## 环境变量

| 变量 | 说明 |
|------|------|
| `OPENAI_API_KEY` | OpenAI API Key |
| `OPENAI_BASE_URL` | 自定义 API 端点 |
| `CODE_HOME` | 自定义配置目录 |
| `CODEX_API_KEY` | exec 模式覆盖 API Key |

---

## 相关文档

- [斜杠命令详解](slash-commands.md)
- [非交互模式](exec.md)
- [配置说明](config.md)
- [认证](authentication.md)
