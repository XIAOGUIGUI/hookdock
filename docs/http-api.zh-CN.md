# 本机 HTTP API

HookDock 将当前连接信息写入 `%APPDATA%\HookDock\runtime.json`。所有接口都需要 `Authorization: Bearer <token>`，请求体最大 2 MiB，服务只监听 `127.0.0.1`。

```json
{ "protocol": 1, "host": "127.0.0.1", "port": 37129, "token": "...", "pid": 1234, "updatedAt": "..." }
```

- `GET /v1/health`：检查服务状态。
- `POST /v1/notifications`：发送普通通知，字段为 `source`、`title`、`body`，可选 `level`、`project`、`url`；返回 `202` 和通知 ID。
- `POST /v1/requests`：发送审批或问题并等待用户响应，最长 24 小时。

普通通知示例：

```json
{
  "source": "generic",
  "title": "构建完成",
  "body": "所有检查已经通过",
  "level": "success",
  "project": "website"
}
```

审批请求使用 `"kind": "approval"`；提问使用 `"kind": "question"`，并可提供 `"options": ["预览环境", "生产环境"]`。响应包含 `id`、`decision`，以及可选的 `reason` 和 `answers`。
