# Local HTTP API

HookDock writes its active endpoint to `%APPDATA%\HookDock\runtime.json`:

```json
{ "protocol": 1, "host": "127.0.0.1", "port": 37129, "token": "...", "pid": 1234, "updatedAt": "..." }
```

Every endpoint requires `Authorization: Bearer <token>`. Requests are limited to 2 MiB and the listener never binds outside loopback.

## Health

```http
GET /v1/health
```

## Notification

```http
POST /v1/notifications
Content-Type: application/json

{
  "source": "generic",
  "title": "Build complete",
  "body": "All checks passed",
  "level": "success",
  "project": "website",
  "url": "https://example.test/build/42"
}
```

Returns `202` with `{ "id": "...", "status": "accepted" }`.

## Approval or question

`POST /v1/requests` keeps the connection open until the user responds or the 24-hour timeout expires.

```json
{
  "source": "generic",
  "kind": "approval",
  "title": "Deploy website?",
  "body": "Deploy commit 8f3a1c to production",
  "action": "deploy",
  "project": "website"
}
```

For a question, use `"kind": "question"` and optionally provide `"options": ["Preview", "Production"]`. The response contains `id`, `decision`, optional `reason`, and optional `answers`.
