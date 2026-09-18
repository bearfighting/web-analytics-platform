# Phase 3 Design — Storage, Page View Processing and Analytics API

> Status: Working design
> Scope: PostgreSQL raw event storage, idempotent Page View processing, minimal Analytics API and end-to-end verification

## 1. Phase 3 定义

Phase 3 在 Phase 2 的 Collector、Event Protocol V1 和 FetchTransport 之上，建立第一个可持久化、可重处理、可查询的 Analytics workflow：

```text
Browser SDK → FetchTransport → Collector
  → PostgreSQL raw_events → Page View Processor
  → PostgreSQL aggregates → Analytics API
```

本阶段只保证 Page View 统计链路正确和可重复执行，不提前实现完整的 Visitor、Session、设备维度或实时基础设施。

## 2. 必须达成

- PostgreSQL 可以通过 Docker Compose 启动，并由 migration 初始化。
- Collector 的合法事件可以写入 PostgreSQL Raw Event Store。
- Raw Event 作为不可变事实保存完整原始 payload。
- 同一 `site_id + event_id` 重复写入不会产生重复 Raw Event。
- Processor 可以读取未处理事件并生成 Page View 聚合。
- Processor 重复执行不会重复累加统计。
- Page View 以事件的 `occurred_at` 按 UTC 日期统计。
- Analytics API 只读取聚合表，不直接暴露数据库。
- 提供 Overview、Timeline 和 Top Pages 查询。
- 完成 Client → Collector → PostgreSQL → Processor → Analytics API 的 E2E workflow。

## 3. 不属于本阶段

- Visitor ID、Visitor 统计、Sessionization、Bounce 和 Engagement。
- Bot Detection、Geo、IP 持久化、User-Agent 解析。
- UTM、Device、Browser、OS 聚合查询。
- Kafka、Redis、复杂队列、分布式锁和 distributed exactly-once。
- Site 管理 API、用户登录和复杂权限系统。
- Dashboard UI；Dashboard 留在 Phase 4。
- BeaconTransport、离线队列和持久化 Browser Buffer。

当前 Event Protocol V1 没有稳定的匿名 Visitor 标识。不能使用 `event_id` 或任意 `context` 字段推导 Visitor，因此 Visitor/Session 必须等匿名身份协议和 SDK 行为单独确定后再实现。

## 4. 实施原则

### 4.1 Contract first

先固定 Storage、Processing 和 Analytics API 契约，再实现数据库和服务。Event Protocol V1 仍然是 Collector 接收事件的唯一跨语言事实来源，本阶段不修改它。

### 4.2 Raw Event 是事实源

Collector 写入的 Raw Event 不被 Processor 原地改写。Processor 通过聚合表生成查询结果，统计算法变化时可以从 Raw Event 重新处理。

```text
raw_events → page_view_daily
           → page_view_routes
```

### 4.3 先保证单进程幂等

Phase 3 只要求单个 Processor 实例稳定运行。通过数据库唯一约束、事务和处理状态保证重复执行安全，不提前引入分布式协调。

### 4.4 时间语义固定为 UTC

Phase 3 的 `from`、`to` 和每日聚合统一使用 UTC。数据库同时记录客户端 `occurred_at` 和 Collector `received_at`。站点时区报表延后设计。

## 5. Storage Contract

### 5.1 Raw Event 写入语义

Storage 必须：

- 写入完整原始事件 JSON。
- 保存 `site_id`、`event_id`、`type`、`schema_version`、`occurred_at` 和服务端 `received_at`。
- 以 `site_id + event_id` 作为幂等键。
- 重复事件不产生新的 Raw Event，也不重复进入 Processor。
- 数据库事务提交成功后，Collector 才返回成功。
- Storage 失败时，Collector 返回现有的 `500 collector_error`。

Phase 2 的 `EventSink::accept` 表达“将 batch 交给 Sink”，不区分新增和重复。Phase 3 保留这个 HTTP 语义：重复事件是成功的幂等 no-op。

### 5.2 `raw_events` 表

首个 migration 至少包含：

```text
id             bigint generated always as identity primary key
site_id        varchar(64) not null
event_id       varchar(26) not null
schema_version integer not null
event_type     varchar(64) not null
occurred_at    timestamptz not null
received_at    timestamptz not null
path           text not null
url            text null
title          text null
referrer       text null
payload        jsonb not null
processed_at   timestamptz null
created_at     timestamptz not null
```

约束和索引：

- `unique (site_id, event_id)`。
- `payload` 保存完整 PageViewEvent，包括未知字段。
- `(site_id, processed_at, id)` 用于 Processor 扫描。
- `(site_id, occurred_at)` 用于重处理和诊断。

`processed_at` 表示该事件已参与当前 Page View 聚合，不是删除标记。Phase 3 不实现 retention。

### 5.3 聚合表

首期只创建：

```text
page_view_daily
- site_id
- day                  date
- page_views           bigint

page_view_routes
- site_id
- day                  date
- path
- page_views           bigint
```

主键分别为 `(site_id, day)` 和 `(site_id, day, path)`。路径使用 Event Protocol 中的 `path` 原值；Phase 3 不做动态路由模式归一化。

## 6. Processing Semantics

Processor 负责读取未处理 Raw Event、按 `occurred_at` 计算 UTC 日期、更新两个聚合表，并在同一事务中标记 Raw Event 已处理。它不修改 Raw Event，也不从 `context` 推导 Visitor、设备或浏览器维度。

单个处理事务：

```text
read raw event → upsert daily aggregate
               → upsert route aggregate
               → set raw_events.processed_at
               → commit
```

事务失败时全部回滚，下一次运行可以重试。迟到事件仍进入其 `occurred_at` 对应的 UTC 日期。复杂窗口修正和多实例 claim/lease 延后。

Processor 至少提供：

```text
processor --once
processor --poll-interval-ms 1000
```

`--once` 用于测试和本地 E2E；默认模式持续轮询未处理事件。

## 7. Analytics API Contract

Analytics API 默认监听 `0.0.0.0:4002`，只读取聚合表。

```http
GET /health
GET /v1/sites/{site_id}/overview?from=2026-09-01&to=2026-09-18
GET /v1/sites/{site_id}/timeline?from=2026-09-01&to=2026-09-18
GET /v1/sites/{site_id}/pages?from=2026-09-01&to=2026-09-18&limit=20
```

Overview：

```json
{
  "site_id": "site_playground",
  "from": "2026-09-01",
  "to": "2026-09-18",
  "page_views": 42
}
```

Timeline：

```json
{
  "site_id": "site_playground",
  "from": "2026-09-01",
  "to": "2026-09-18",
  "items": [{ "day": "2026-09-01", "page_views": 12 }]
}
```

Top Pages：

```json
{
  "site_id": "site_playground",
  "from": "2026-09-01",
  "to": "2026-09-18",
  "items": [{ "path": "/", "page_views": 20 }]
}
```

API 规则：

- `from`、`to` 使用 ISO date，按 UTC 解释，且为闭区间。
- `from > to` 返回 `400 invalid_date_range`。
- `limit` 默认 20，最大 100，非法值返回 `400 invalid_limit`。
- 无数据返回 `200`、`page_views: 0` 或空 `items`，不返回 `404`。
- 结果按 `page_views desc`，相同数量按稳定顺序排序。
- site 数据必须隔离；未知 site 可以返回空结果，不暴露站点存在性。
- 数据库不可用时 `/health` 返回非 2xx，且不泄露内部错误。

## 8. Rust Workspace Structure

Phase 3 在现有 workspace 中新增实际服务：

```text
services/
├── collector/
├── processor/
└── analytics-api/
```

初期领域和 PostgreSQL 代码放在服务内部 module。只有需要被多个服务复用的实际类型出现后，才提取 `crates/analytics-domain` 或 `crates/analytics-storage`，不提前创建空 crate。

## 9. Docker Compose 与配置

继续使用 profiles：

```text
default     Router Playground
backend     Collector
storage     PostgreSQL
processing  Processor 和 Analytics API
dashboard   Phase 4 Dashboard
```

服务：`postgres :5432`、`collector :4001`、`processor` background worker、`analytics-api :4002`。

建议配置：

```text
DATABASE_URL
ANALYTICS_API_HOST
ANALYTICS_API_PORT
PROCESSOR_POLL_INTERVAL_MS
```

完整 workflow 应等待 PostgreSQL、Collector `/health` 和 Analytics API `/health` 通过。数据库密码只通过 `.env` 或本地环境注入。

## 10. PR 划分

### PR1 — Storage and Analytics Contracts

- 固定 Raw Event、聚合、幂等和 UTC 语义。
- 固定 Analytics API 请求、响应和错误契约。
- 增加 Processor 输入、API 输出和 E2E fixtures。
- 更新 roadmap、monorepo design 和 getting started 链接。

不创建 PostgreSQL 服务或 Processor 实现。

### PR2 — PostgreSQL Raw Event Storage

- 添加 PostgreSQL Compose service 和 healthcheck。
- 添加 migration runner 及 `raw_events` migration。
- 将 PostgreSQL Sink 接入 Collector。
- 实现 `(site_id, event_id)` 幂等写入。
- 保留 InMemory Sink，增加 Storage integration tests。

验收：`FetchTransport → Collector → PostgreSQL raw_events`。

### PR3 — Idempotent Page View Processor

- 添加两个聚合表 migration。
- 创建 Processor service，支持 `--once` 和持续轮询。
- 实现 Page View 聚合和事务内 processed 标记。
- 增加重复执行、迟到事件、空数据和失败回滚测试。
- 添加 Processor container 和 `processing` profile。

验收：`raw_events → Processor → aggregates`。

### PR4 — Minimal Analytics API

- 创建 `services/analytics-api`。
- 实现 `/health`、`/overview`、`/timeline`、`/pages`。
- 实现日期范围、limit、空数据和错误响应。
- 只从聚合表查询并增加 integration tests。

不创建 Dashboard UI。

### PR5 — End-to-end Workflow

- 用 Playground 或可复现 fixture 发送 Page View。
- 验证 Collector 写入 PostgreSQL。
- 运行 Processor。
- 查询 API 并比对预期 JSON。
- 增加重复事件、多页面、多个 site 和空结果场景。
- 更新 README、getting started 和 Docker 文档。

## 11. 测试策略

Storage tests：合法事件写入并保留完整 payload；`received_at` 与 `occurred_at` 均保存；相同 `site_id + event_id` 不重复插入；不同 site 隔离；数据库错误不返回成功；非法请求不写入数据库。

Processor tests：单事件和多路径聚合正确；重复运行不重复累加；迟到事件进入正确 UTC 日期；事务失败时聚合和 processed 标记一起回滚；无未处理事件时安全退出。

Analytics API tests：Overview、Timeline、Top Pages 正确；日期范围闭区间；非法日期和 limit 有稳定错误；空范围返回 200；site 隔离；不依赖 Raw Event 查询。

E2E fixtures：

```text
single-page-view.json
multi-page-navigation.json
duplicate-events.json
late-event.json
multi-site-isolation.json
empty-date-range.json
```

每个 fixture 同时记录 Raw Events、Processor 执行方式、预期聚合结果和预期 API response。

## 12. 工程脚本

继续使用：

```text
pnpm check
pnpm test
pnpm build
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
docker compose config
docker compose --profile backend --profile storage --profile processing config
```

建议增加 `pnpm db:migrate`、`pnpm test:integration` 和 `pnpm e2e:analytics`。PR2 选择并固定 migration 工具后，再实现这些脚本，不同时维护多套 migration 流程。

## 13. Phase 3 验收清单

### Contract

- [ ] Raw Event storage contract、幂等键、UTC 日期语义已固定。
- [ ] Analytics API contract 已固定。
- [ ] E2E fixtures 已存在。

### Storage

- [ ] PostgreSQL 可以通过 Compose 启动。
- [ ] Migration 可以在空数据库执行。
- [ ] Collector 可以写入 `raw_events`。
- [ ] 完整 payload 被保留。
- [ ] 重复事件不会重复写入。

### Processing

- [ ] Processor 可以执行一次性 batch 和持续轮询。
- [ ] Daily、route Page View aggregate 正确。
- [ ] 重复执行不会重复计数。
- [ ] 处理失败可以安全重试。

### Analytics API

- [ ] `/health`、Overview、Timeline、Top Pages 可用。
- [ ] 日期、limit、空数据和错误状态有测试。
- [ ] API 不直接暴露数据库。

### End-to-end

- [ ] SDK 可以向 Collector 发送事件。
- [ ] 事件可以进入 PostgreSQL。
- [ ] Processor 可以生成聚合。
- [ ] API 可以返回预期结果。
- [ ] 多 site 数据隔离。
- [ ] Docker Compose workflow 可以复现。

## 14. Phase 3 退出条件

以下链路稳定运行后进入 Phase 4。Visitor、Session 和 Browser Dimensions 的语义研究进入 Phase 5，正式实现进入 Phase 6：

```text
analytics-browser → FetchTransport → Collector
  → PostgreSQL raw_events → Page View Processor
  → PostgreSQL aggregates → Analytics API
```

并且新环境可以通过 Compose 启动全部服务；重复事件不会造成重复 Raw Event 或 Page View 统计；Processor 可以重试；API 可以返回 Page Views、Timeline 和 Top Pages；E2E fixture 可以自动验证完整 workflow；Visitor、Session、Browser Dimensions 没有被伪造实现，也没有改变 Event Protocol V1 语义。

## 15. 后续允许调整的内容

可以调整 migration 工具、PostgreSQL driver、连接池、Processor 批量大小、轮询间隔、retention、API JSON 字段、claim/lease 和聚合索引。

以下边界不应改变：Raw Event 是不可变事实源；`site_id + event_id` 具有幂等语义；Processor 与 Collector 解耦；API 不直接读取数据库；Phase 3 只交付 Page View 基础统计；Visitor、Session 和 Browser Dimensions 不在没有数据契约的情况下提前实现。
