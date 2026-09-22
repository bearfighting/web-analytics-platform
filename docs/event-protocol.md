# Unified Event Protocol

这是 Client SDK 和 Backend 之间的统一初始跨语言契约。`schema_version: 1`
已经包含 Visitor ID 和 Browser Context；V1/V2 是此前的开发阶段实现方式，
不再作为 runtime compatibility boundary。

## PageViewEvent

Page View 事件使用 JSON Schema 2020-12 描述，字段使用 `snake_case`：

| 字段                     | 必填 | 规则                          |
| ------------------------ | ---- | ----------------------------- |
| `schema_version`         | 是   | 固定为 `1`                    |
| `event_id`               | 是   | 26 位 ULID                    |
| `type`                   | 是   | 固定为 `page_view`            |
| `site_id`                | 是   | 1–64 位字母、数字、`_` 或 `-` |
| `occurred_at`            | 是   | 非负 Unix milliseconds        |
| `path`                   | 是   | 以 `/` 开头，最长 2048 字符   |
| `url`                    | 否   | URI，最长 4096 字符           |
| `title`                  | 否   | 最长 512 字符                 |
| `referrer`               | 否   | URI reference，最长 4096 字符 |
| `visitor_id`             | 否   | 站点范围内的匿名 UUID v4      |
| `context_schema_version` | 条件 | 有 `context` 时固定为 `1`     |
| `context`                | 否   | 完整 Browser Context 对象     |

顶层未知字段允许存在，为未来的非破坏性扩展保留空间。

## EventBatch

Event Batch 包含 1–100 条 PageViewEvent：

```json
{
  "schema_version": 1,
  "events": [
    {
      "schema_version": 1,
      "event_id": "01J00000000000000000000000",
      "type": "page_view",
      "site_id": "site_example",
      "occurred_at": 1760000000000,
      "path": "/about"
    }
  ]
}
```

Batch 必须非空，且每一项都必须符合统一 PageViewEvent。Batch 顶层未知字段同样允许存在。

## 版本策略

- 所有事件和 batch 都包含 `schema_version`。
- 当前正式协议只接受版本号 `1`。
- `context_schema_version` 描述 Browser Context 语义版本，不是 Event Protocol rollout 版本。
- 有 Context 时必须同时提供 `context_schema_version`；有 `context_schema_version` 时必须同时提供 Context。
- 客户端不得发送 `session_id`；Session 由 Processor 派生。
- Collector 拒绝 `occurred_at` 晚于 `received_at + 5 minutes` 的事件。
- 新事件类型和破坏性修改使用新的 schema version。
- PR3 不实现版本转换或旧版本迁移。
- `CustomEvent`、Web Vital、错误和转化事件留到后续阶段。

## 文件与校验

正式 Schema 位于 `protocol/events/schemas/`，示例位于
`protocol/events/examples/`，合法和非法测试数据位于
`protocol/events/fixtures/`。

运行协议校验：

```bash
pnpm protocol:validate
```

该命令使用 Ajv 和 `ajv-formats` 校验所有 fixtures，并确认合法和非法 fixture 的预期结果都正确。协议校验也包含在 `pnpm test` 中。
