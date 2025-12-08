# TUI 设置面板

Every Code TUI 的全屏设置面板，可在不离开聊天的情况下修改模型、主题、Auto Drive 默认值、智能体、通知等。

## 打开与导航
- `/settings` 打开总览；`/settings <section>` 直接跳转到指定区域（如下）。`/auto settings` 与 `/update` 会跳到各自区域。
- 按键：`↑/↓` 或 `j/k` 移动，`Tab/Shift+Tab` 切换区域，`Home/End` 跳到列表首尾。Enter 打开/确认；Esc 关闭当前区域，再按 Esc 关闭面板。`?` 切换内联帮助。允许时粘贴会传递给当前区域。
- 面板是模态的：显示时阻塞聊天输入。重新打开时会记住上次激活的区域（`pending_settings_return`）。

## 持久化
- 更改会写入 `CODE_HOME/config.toml`（如目录不存在会提示警告，变更仅作用当前会话）。
- 访问模式可按工作区保存；其他设置为全局，除非配置文件为项目覆盖。
- Agent 与 MCP 的修改也存储在同一配置目录。

## 区域
- **Model**：选择默认聊天模型与推理力度。
- **Theme**：选择主题与 spinner；即时生效。
- **Updates**：查看升级通道/状态。`/update` 在运行安装前会打开此处。
- **Agents**：查看内置/自定义智能体，启用/禁用、强制只读、添加智能体指令。打开子智能体编辑器可配置 `/plan`/`/solve`/`/code` 或自定义斜杠命令。
- **Prompts**：编辑保存的提示片段。
- **Auto Drive**：设置审查/智能体/QA/交叉检查开关、继续模式（manual/immediate/ten-seconds/sixty-seconds）、模型覆盖或“use chat model”。更新会应用到活动运行。
- **Review**：选择审查模型（或复用聊天）、切换自动解决、设置自动解决尝试次数上限。
- **Planning**：为规划轮次选择模型/力度或复用聊天模型。
- **Validation**：切换验证分组与工具；查看安装状态；触发安装帮助。
- **Limits**：只读查看限流与上下文/自动压缩使用情况。
- **Chrome**：在浏览器连接失败时显示；可选择重试、使用临时配置、切换到内置浏览器或取消。
- **MCP**：启用/禁用 MCP 服务器。
- **Notifications**：全局切换或设置筛选。`/notifications on|off|status` 也会跳到此处。

## 面板生命周期
- 打开时所有按键都通过 `handle_settings_key`；输入框/历史会忽略输入，直到关闭。
- 按 Esc 时会先关闭帮助层（`?`），再关闭主面板。
- 各区域通过自身内容结构标记完成；当区域报告 `is_complete`（如已选择 Chrome 选项）时面板关闭。

## 作用域提示
- 全局默认值存于 `CODE_HOME/config.toml`。
- 工作区覆盖在 setter 接受 `cwd`（访问模式）或存在项目级配置文件时生效。UI 始终渲染合并后的有效值。
- 智能体命令与 MCP 服务器存储在 `CODE_HOME` 下，对所有工作区生效，除非被项目配置覆盖。

## 命令
- `/settings [section]`
- `/auto settings`
- `/update`（或 `/update settings`）
- `/notifications [on|off|status]`
