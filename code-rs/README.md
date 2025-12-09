# Every Code CLI（Rust 实现）

我们提供零依赖的原生可执行文件，直接运行即可体验终端版 Every Code。

## 安装 Every Code

```bash
npm install -g @just-every/code
code   # 若命令被占用，可使用 coder
```

更多安装方式（Homebrew、直接下载预编译包）见 [`docs/install.md`](../docs/install.md)。

## Rust CLI 有什么不同

Rust 版是当前维护的主 CLI，功能覆盖并超越早期的 TypeScript 版本。

### 配置

Rust CLI 使用 `config.toml` 而非 `config.json`。详细选项见 [`docs/config.md`](../docs/config.md)。

## 汉化维护原则

- 汉化改动仅落在 `code-rs/`，不改镜像的 `codex-rs/`，方便跟上游同步。
- 默认界面仍为英文；开启 `ui_locale = "zh-CN"` 后显示中文，缺失条目回退英文，避免功能回归。
- 字符串集中管理（单一表/模块），上游增量改文案时只需补键，降低合并冲突。
- 所有汉化变更需继续通过 `./build-fast.sh`，遇到 warning 视为失败。

### Model Context Protocol 支持

Rust CLI 默认作为 MCP 客户端启动，按 `config.toml` 中的 [`mcp_servers`](../docs/config.md#mcp_servers) 配置连接服务器。也可用 `code mcp-server` 让 CLI 作为 MCP **服务器** 运行，并通过 [`@modelcontextprotocol/inspector`](https://github.com/modelcontextprotocol/inspector) 体验：

```bash
npx @modelcontextprotocol/inspector code mcp-server
```

使用 `code mcp` 可添加/列出/查看/删除 `config.toml` 中声明的 MCP 服务器启动项。

### 通知

可在 `config.toml` 的 `[notify]` 配置一个脚本，在智能体完成一轮时触发。示例见 [`docs/config.md#notify`](../docs/config.md#notify)，其中演示了如何在 macOS 上借助 [terminal-notifier](https://github.com/julienXX/terminal-notifier) 发送桌面提醒。

### `code exec` 非交互模式

在无头/自动化场景下使用 `code exec "PROMPT"`；也可从 stdin 读取提示。默认输出直接打印到终端。设置 `RUST_LOG` 可查看更多内部日志。

### 用 `@` 搜索文件

在输入框键入 `@` 可在工作区内模糊搜索文件名。上下键选择，Tab 或 Enter 将 `@` 替换为选中的路径，Esc 取消。

### Esc–Esc 编辑上一条消息

输入框为空时按 Esc 进入“回溯”模式；再次按 Esc 打开转录预览，持续按可回退更早的用户消息；Enter 确认后从该点分叉对话并预填输入框。预览页脚会提示 `Esc edit prev` 以表明正在编辑。

### `--cd`/`-C` 参数

无需先 `cd`；`code --cd /path/to/project` 可直接指定工作根。新会话开始时在 TUI 页脚检查 **workdir** 即可确认。

### Shell 自动补全

生成 shell 补全脚本：

```bash
code completion bash
code completion zsh
code completion fish
```

### 试验沙箱行为

想测试命令在沙箱下的表现，可用：

```bash
# macOS
code sandbox macos [--full-auto] [COMMAND]...

# Linux
code sandbox linux [--full-auto] [COMMAND]...
```

### 通过 `--sandbox` 选择策略

Rust CLI 暴露 `--sandbox`（`-s`）以无需 `-c/--config` 也能快速选择沙箱策略：

```bash
code --sandbox read-only          # 默认只读
code --sandbox workspace-write    # 工作区可写但默认禁网
code --sandbox danger-full-access # 危险：关闭沙箱
```

同样可在 `~/.code/config.toml`（也读取 `~/.codex/config.toml`）中持久化，例如：

```toml
sandbox_mode = "workspace-write"

[sandbox_workspace_write]
allow_git_writes = false
```

### TUI 防截断兜底

若历史最后一行偶发被截断，TUI 会启用受控底部留白，并在高度可能与视口齐平时添加 1–2 行 overscan，减少流式过程中末行消失的概率。调试布局时可设 `RUST_LOG=debug` 观察兜底是否触发。

### 调试 Virtual Cursor

在真实浏览器内调试虚拟光标的移动/取消行为可用以下控制台片段：

- 禁用 clickPulse 并放大动画时长：

  `window.__vc && (window.__vc.clickPulse = () => (console.debug('[VC] clickPulse disabled'), 0), window.__vc.setMotion({ engine: 'css', cssDurationMs: 10000 }))`

- 包装 `moveTo` 以记录重复调用的序号与时间间隔：

  `(() => { const vc = window.__vc; if (!vc || vc.__wrapped) return; const orig = vc.moveTo; let seq=0, last=0; vc.moveTo = function(x,y,o){ const now=Date.now(); console.debug('[VC] moveTo call',{seq:++seq,x,y,o,sincePrevMs:last?now-last:null}); last=now; return orig.call(this,x,y,o); }; vc.__wrapped = true; console.debug('[VC] moveTo wrapper installed'); })();`

- 触发测试移动（可调整坐标）：

  `window.__vc && window.__vc.moveTo(200, 200)`

## 代码结构

本目录是一个 Cargo workspace，包含若干核心 crate：

- [`core/`](./core)：Every Code 的业务逻辑，未来希望作为通用库复用。
- [`exec/`](./exec)：无头 CLI，用于自动化场景。
- [`tui/`](./tui)：基于 [Ratatui](https://ratatui.rs/) 的全屏终端 UI。
- [`cli/`](./cli)：多功能入口，提供以上子命令。
