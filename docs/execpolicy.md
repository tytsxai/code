# Execpolicy 快速上手

Every Code 可以在运行 shell 命令前套用你自定义的规则执行策略。策略保存在 `~/.code/policy` 下的 Starlark `.codexpolicy` 文件里（兼容读取 `~/.codex/policy`）。

## 创建策略

1. 创建策略目录：`mkdir -p ~/.code/policy`。
2. 在该目录添加一个或多个 `.codexpolicy` 文件。Code 启动时会自动加载其中的所有 `.codexpolicy`。
3. 通过 `prefix_rule` 声明需要允许、提示或禁止的命令：

```starlark
prefix_rule(
    pattern = ["git", ["push", "fetch"]],
    decision = "prompt",  # allow | prompt | forbidden
    match = [["git", "push", "origin", "main"]],  # 必须匹配的示例
    not_match = [["git", "status"]],              # 不应匹配的示例
)
```

- `pattern` 是按顺序匹配的命令 token；用嵌套列表表示可选项（如同时匹配 `push` 与 `fetch`）。
- `decision` 设定严重程度；多条规则命中时取最严格的结果（forbidden > prompt > allow）。
- `match` 与 `not_match` 相当于可选单元测试。Code 加载策略时会校验，示例行为异常会及时反馈。

上述规则表示：当 Code 想运行以 `git push` 或 `git fetch` 开头的命令时，会先询问用户确认。

## 预览决策

使用 `code execpolicy check` 子命令可在保存前预览决策（语法详见 [`codex-execpolicy` README](../code-rs/execpolicy/README.md)）：

```shell
code execpolicy check --policy ~/.code/policy/default.codexpolicy git push origin main
```

通过多个 `--policy` 参数可测试多文件组合效果，`--pretty` 输出格式化 JSON。更完整的语法说明见 [`code-rs/execpolicy` README](../code-rs/execpolicy/README.md)。

## 状态

`execpolicy` 命令仍处于预览阶段，未来 API 可能有破坏性变更。
