# Codex SDK

在工作流和应用里嵌入 Codex 智能体。TypeScript SDK 对随包提供的 `codex` 可执行文件做薄封装，通过 stdin/stdout 交换 JSONL 事件。

## 安装

```bash
npm install @openai/codex-sdk
```

需要 Node.js 18+。

## 快速上手

```typescript
import { Codex } from "@openai/codex-sdk";

const codex = new Codex();
const thread = codex.startThread();
const turn = await thread.run("Diagnose the test failure and propose a fix");

console.log(turn.finalResponse);
console.log(turn.items);
```

在同一 `Thread` 上重复调用 `run()` 可继续对话：

```typescript
const nextTurn = await thread.run("Implement the fix");
```

### 流式响应

`run()` 会缓冲事件直到轮次完成。若需要实时处理工具调用、流式回答、文件 diff，请使用 `runStreamed()`，它返回一个异步迭代器：

```typescript
const { events } = await thread.runStreamed("Diagnose the test failure and propose a fix");

for await (const event of events) {
  switch (event.type) {
    case "item.completed":
      console.log("item", event.item);
      break;
    case "turn.completed":
      console.log("usage", event.usage);
      break;
  }
}
```

### 恢复已有线程

线程会持久化在 `~/.codex/sessions`。如果丢失了内存中的 `Thread` 对象，可用 `resumeThread()` 继续：

```typescript
const savedThreadId = process.env.CODEX_THREAD_ID!;
const thread = codex.resumeThread(savedThreadId);
await thread.run("Implement the fix");
```

### 工作目录控制

Codex 默认在当前工作目录运行，并要求该目录是 Git 仓库。若需跳过 Git 检查，在创建线程时传 `skipGitRepoCheck`：

```typescript
const thread = codex.startThread({
  workingDirectory: "/path/to/project",
  skipGitRepoCheck: true,
});
```
