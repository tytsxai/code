## 安装与构建

### 系统要求

| 要求                      | 详情                                                              |
| ------------------------- | ----------------------------------------------------------------- |
| 操作系统                  | macOS 12+、Ubuntu 20.04+/Debian 10+，或通过 WSL2 的 Windows 11     |
| Git（可选，推荐）         | 2.23+，方便使用内置 PR 工具                                       |
| 内存                      | 至少 4 GB（推荐 8 GB）                                            |

### DotSlash

GitHub Release 包含名为 `code` 的 [DotSlash](https://dotslash-cli.com/) shim。把 DotSlash 文件检入仓库即可在跨平台固定同一二进制。

### 从源码构建

```bash
# 克隆仓库并进入工作区
 git clone https://github.com/just-every/code.git
 cd code

# 如有需要，安装 Rust 工具链
 curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
 source "$HOME/.cargo/env"

# 构建全部组件（CLI、TUI、MCP 服务器），与 CI 检查一致
 ./build-fast.sh

# 用示例提示启动 TUI
 ./target/debug/code -- "explain this codebase to me"
```

> [!NOTE]
> 项目将编译警告视为错误。唯一必需的本地检查是 `./build-fast.sh`；除非特别要求，否则不要运行 `rustfmt`/`clippy`。
