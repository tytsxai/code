## 快速上手

### CLI 用法

| 命令              | 作用                             | 示例                          |
| ----------------- | -------------------------------- | ----------------------------- |
| `code`            | 交互式 TUI                       | `code`                        |
| `code "..."`     | 交互式 TUI 的初始提示            | `code "fix lint errors"`     |
| `code exec "..."` | 非交互“自动化模式”               | `code exec "explain utils.ts"` |

关键参数：`--model/-m`、`--ask-for-approval/-a`。

### 使用提示作为输入运行

也可以直接给 Code 一个提示并运行：

```shell
code "explain this codebase to me"
```

```shell
code --full-auto "create the fanciest todo-list app"
```

就是这样——Code 会脚手架代码、在沙箱里运行、安装缺失依赖并展示实时结果。审批后会将改动写入你的工作目录。

### 示例提示

下面是几段可直接复制的示例，把引号里的文本换成你的任务即可。

| ✨  | 你输入的内容                                                          | 会发生什么                                                                 |
| --- | ------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| 1   | `code "Refactor the Dashboard component to React Hooks"`            | Code 重写类组件、运行 `npm test` 并展示 diff。                             |
| 2   | `code "Generate SQL migrations for adding a users table"`           | 推断 ORM，创建迁移文件并在沙箱数据库里运行。                               |
| 3   | `code "Write unit tests for utils/date.ts"`                         | 生成测试、执行并迭代直到通过。                                             |
| 4   | `code "Bulk-rename *.jpeg -> *.jpg with git mv"`                    | 安全重命名文件并更新引用。                                                 |
| 5   | `code "Explain what this regex does: ^(?=.*[A-Z]).{8,}$"`           | 给出分步的人类可读解释。                                                   |
| 6   | `code "Carefully review this repo, and propose 3 high impact well-scoped PRs"` | 在当前代码库中提出高影响、范围明确的 PR 建议。                              |
| 7   | `code "Look for vulnerabilities and create a security review report"`        | 查找并解释安全问题。                                                       |

### 使用 AGENTS.md 记忆

你可以通过 `AGENTS.md` 为 Every Code 提供额外指令。Code 会按以下路径自上而下查找并合并：

1. `~/.code/AGENTS.md` —— 个人全局指南（若存在旧版 `~/.codex/AGENTS.md` 也会读取）
2. 仓库根目录的 `AGENTS.md` —— 共享项目备注
3. 当前工作目录的 `AGENTS.md` —— 子目录/功能的具体说明

更多用法参见 [AGENTS.md 官方文档](https://agents.md/)。

### 提示与快捷键

#### 用 `@` 搜索文件

输入 `@` 会在工作区根目录触发模糊文件名搜索。用上下键选择，Tab 或 Enter 将 `@` 替换为选中的路径。Esc 取消搜索。

#### 图像输入

直接在输入框粘贴图片（Ctrl+V / Cmd+V）即可附加。CLI 也可通过 `-i/--image`（逗号分隔）附加文件：

```bash
code -i screenshot.png "Explain this error"
code --image img1.png,img2.jpg "Summarize these diagrams"
```

#### Esc–Esc 编辑上一条消息

当输入框为空时按 Esc 进入“回溯”模式。再按 Esc 打开转录预览，高亮最近的用户消息；继续按 Esc 可回到更早的消息。按 Enter 确认后，Code 会从该点分叉对话、裁剪可见记录，并将选中的用户消息预填到输入框供你修改再提交。

在转录预览中，页脚会显示 `Esc edit prev` 以提示正在编辑。

#### Shell 自动补全

生成 shell 补全脚本：

```shell
code completion bash
code completion zsh
code completion fish
```

#### `--cd`/`-C` 参数

有时不方便先 `cd` 到希望作为“工作根”的目录。`code` 支持 `--cd` 选项，可直接指定任意目录。你可以在 TUI 开始时查看 **workdir** 以确认 Code 已应用 `--cd`。
