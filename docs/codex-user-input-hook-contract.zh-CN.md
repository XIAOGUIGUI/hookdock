# Codex `request_user_input` 外部 Hook 需求

## 1. 目标

让 Codex 的 `request_user_input` 在显示原生终端提问界面之前，先通过一个可阻塞的外部 Hook 把问题交给 HookDock。用户可以在 HookDock 中选择选项或输入答案；HookDock 把结果回传给 Codex 后，Codex 继续当前工具调用。

这个改动只扩展 Codex 的 Hook 能力，不改变现有内部 `EventMsg::RequestUserInput` 和 app-server 的回答协议。

## 2. 当前问题

Codex 0.153.4 在 `codex-rs/core/src/session/mod.rs` 的 `request_user_input` 中直接发送内部 `EventMsg::RequestUserInput`，然后等待内部 oneshot。`codex-rs/protocol/src/protocol.rs` 的外部 `HookEventName` 没有对应事件，所以 `.codex/hooks.json` 无法观察或回答这类问题。

不能复用 `PermissionRequest`：它表达的是允许/拒绝工具执行，而 `request_user_input` 的结果是按问题 ID 返回一个或多个字符串。混用会让 Hook 输出结构和失败回退语义都不清晰。

## 3. 新增事件

新增外部 Hook 事件：

```text
UserInputRequest
```

要求：

- 在 `HookEventName` 中增加该枚举值，并同步生成的 app-server schema、TypeScript/Python 类型和相关快照。
- `.codex/hooks.json` 可以使用 `hooks.UserInputRequest` 配置命令 Hook。
- 这是同步、可阻塞事件；执行器必须等待命令退出并读取 stdout。
- 该事件不使用 matcher。存在 matcher 时可以忽略，不能按工具名过滤掉请求。
- 现有 Hook 的行为和输出格式保持不变。

HookDock 安装的配置形态如下：

```json
{
  "hooks": {
    "UserInputRequest": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"C:\\...\\hookdock-hook.exe\" --source codex",
            "timeout": 86400
          }
        ]
      }
    ]
  }
}
```

## 4. Codex 传给 Hook 的 stdin

命令 stdin 必须是一个 JSON 对象，字段如下：

```json
{
  "session_id": "0199...",
  "turn_id": "turn-...",
  "cwd": "C:\\work\\project",
  "transcript_path": null,
  "hook_event_name": "UserInputRequest",
  "tool_name": "request_user_input",
  "call_id": "call_abc123",
  "questions": [
    {
      "id": "environment",
      "header": "Environment",
      "question": "Where should I deploy?",
      "options": [
        {
          "label": "Staging",
          "description": "Deploy to the test environment"
        },
        {
          "label": "Production",
          "description": "Deploy to the live environment"
        }
      ],
      "isOther": true,
      "isSecret": false
    }
  ],
  "isBlocking": true
}
```

字段规则：

| 字段 | 要求 |
| --- | --- |
| `session_id` | Codex thread ID，必填 |
| `turn_id` | 当前 turn ID，必填 |
| `cwd` | 当前 turn 的工作目录，必填 |
| `transcript_path` | 有值时传字符串，没有时传 `null` |
| `hook_event_name` | 固定为 `UserInputRequest` |
| `tool_name` | 固定为 `request_user_input` |
| `call_id` | 当前工具调用 ID，必填且在并发请求中唯一 |
| `questions` | 原样保留 `RequestUserInputQuestion` 的 ID、标题、问题、选项、`isOther` 和 `isSecret` |
| `isBlocking` | 固定为 `true` |

不要把 `header` 当作问题 ID。答案关联始终使用 `questions[].id`。

## 5. Hook stdout

Hook 有两种有效结果。

### 5.1 已回答

stdout 直接使用 Codex 已有的 `RequestUserInputResponse` 结构：

```json
{
  "answers": {
    "environment": {
      "answers": ["Staging"]
    }
  }
}
```

- `answers` 的 key 必须对应 `questions[].id`。
- 每个问题的内层 `answers` 至少包含一个非空字符串。
- 多选或需要返回多个值时，放入同一个字符串数组。
- Codex 必须验证所有问题都有答案。验证通过后，直接把它作为当前 `request_user_input` 的结果，不再发送内部原生提问事件。

### 5.2 未处理

```json
{}
```

`{}` 表示外部 Hook 没有回答，Codex 必须继续现有原生流程。它不是拒绝，也不是空答案。

## 6. 执行顺序和失败回退

建议在 `Session::request_user_input` 中注册 elicitation 之后、创建内部 pending input 和发送 `EventMsg::RequestUserInput` 之前执行 Hook：

```text
收到 request_user_input
  → 执行所有匹配的 UserInputRequest Hook
  → 得到一份完整且合法的 RequestUserInputResponse？
      是：直接返回该答案
      否：执行现有 pending input + EventMsg::RequestUserInput + oneshot 流程
```

以下情况都必须进入原生流程，不能结束 turn，也不能让工具调用永久卡住：

- 没有配置 `UserInputRequest` Hook；
- HookDock 没启动或命令无法启动；
- Hook 超时、非零退出、被中断；
- stdout 为空、不是合法 JSON、为 `{}` 或结构不合法；
- 少了任何问题 ID，或答案数组为空/只含空白；
- 用户在 HookDock 中点击取消。

如果配置了多个 Hook，采用现有同步 Hook 的确定性顺序。第一份完整合法答案可以结束处理；只有日志/返回 `{}` 的 Hook 不应阻止后续 Hook。若现有 Hook 执行器不支持这一语义，至少要保证所有 Hook 都未给出合法答案时能回退原生 UI。

## 7. 并发和生命周期

- 不允许用 thread ID 作为单个待回答请求的唯一 key；同一 thread 内可能存在多个并发工具调用。
- `call_id` 是请求关联 ID，回答只应用到对应调用。
- turn 被取消、线程结束或 Codex 退出时，应取消正在等待的 Hook 子进程，并清理 pending 状态。
- 外部 Hook 已返回答案后，不得再产生同一个问题的内部 `EventMsg::RequestUserInput`，避免 HookDock 和 TUI 同时出现两个提问框。

## 8. 敏感输入

- `isSecret: true` 必须原样发送给 HookDock。
- Codex 不得把 Hook stdout 中的敏感答案写入普通日志、错误信息或 tracing 字段。
- 解析失败日志只能记录事件、`call_id` 和错误类别，不能记录完整 stdout。
- HookDock 会用密码输入框展示此类问题，回答只存在于内存中的阻塞请求，不进入通知正文或事件历史。

## 9. 建议改动位置

以 Codex 0.153.4 为基准，至少检查：

- `codex-rs/protocol/src/protocol.rs`：`HookEventName` 增加 `UserInputRequest`；
- `codex-rs/protocol/src/request_user_input.rs`：复用问题和回答类型，必要时新增外部 Hook payload 类型；
- `codex-rs/core/src/session/mod.rs`：在 `request_user_input` 的原生事件前接入 Hook 和回退；
- Codex Hook 的配置解析、matcher、命令执行和输出解析模块：注册新事件及其同步输出；
- `codex-rs/app-server-protocol/schema/`、SDK 生成文件和快照：同步枚举变化。

实现时优先复用现有 Hook 命令执行器的环境变量、工作目录、信任审查、超时和进程取消机制，不要在 `request_user_input` 中另起一套 `Command` 执行逻辑。

## 10. 验收标准

Codex 侧至少覆盖以下测试：

1. 配置 `UserInputRequest` 后，Hook stdin 包含完整问题元数据和 `call_id`。
2. Hook 返回单选答案时，`request_user_input` 直接返回，内部 `EventMsg::RequestUserInput` 不发送。
3. 多个问题和多值答案按问题 ID 正确解析。
4. `{}`、空 stdout、非法 JSON、缺少问题、空答案、非零退出、超时均回退原生 UI。
5. 未配置新 Hook 时，行为与 0.153.4 完全一致。
6. 两个并发 `call_id` 的答案不会串线。
7. turn 取消后 Hook 子进程和等待状态被清理。
8. `isSecret` 答案不出现在测试日志/trace 中。
9. 原有 `PermissionRequest`、`PreToolUse` 和生命周期 Hook 回归测试保持通过。

## 11. 联调方法

1. 构建带此改动的 Codex。
2. 安装支持 `UserInputRequest` 的 HookDock，并在设置中重新安装 Codex Hook。
3. 在 HookDock Terminal 的新 Pane 中运行 Codex。
4. 触发 `request_user_input`。
5. 确认 HookDock 显示问题，选项和自由输入可提交；提交后 Codex 继续执行且 TUI 不出现重复问题。
6. 再次触发问题后退出 HookDock 或点击取消，确认 Codex 原生提问界面立即接管。
