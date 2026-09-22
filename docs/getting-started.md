# Getting Started

## Requirements

- Node.js 22 LTS
- pnpm 11
- Rust 1.96.0 with Cargo

Phase 0 和 Phase 1 不需要 Rust、Cargo、PostgreSQL 或其他后端依赖。Phase 2 Collector 需要 Rust；Phase 3 Storage 需要 Docker 和 PostgreSQL。

## Install

在仓库根目录执行：

```bash
pnpm install
```

## Run the Playground

```bash
pnpm dev
```

默认访问：

```text
http://localhost:3000
```

当前 Playground 提供常见 Next.js App Router 导航场景和 Navigation Debug Panel。

可测试页面：

```text
/
/about
/products/example
/search?q=test
/nested
/nested/child
```

可测试操作：

```text
<Link>
router.push()
router.replace()
router.back()
router.forward()
search params
hash
```

## Run the Dashboard

Phase 4 的 Dashboard 运行在独立的 Next.js app 中，已接入 Overview、Timeline、Top Pages、站点选择和日期范围查询。Dashboard 只通过服务端 Query Client 请求 Analytics API。

```bash
DASHBOARD_SITES=site_playground,site_alpha \
DASHBOARD_DEFAULT_SITE=site_playground \
ANALYTICS_API_URL=http://localhost:4002 \
pnpm --filter @web-analytics/dashboard dev
```

默认访问：

```text
http://localhost:3000/dashboard
```

Dashboard 支持以下 URL 参数：

```text
/dashboard?site_id=site_playground&from=2026-09-01&to=2026-09-18
```

`DASHBOARD_SITES` 和 `DASHBOARD_DEFAULT_SITE` 必须配置且默认站点必须属于允许列表。`ANALYTICS_API_URL` 必须是绝对的 HTTP(S) URL，例如 `http://localhost:4002`。Dashboard 通过服务端 Query Client 请求 Overview、Timeline 和 Top Pages；页面不会直接从浏览器请求 Analytics API。

以 Compose 启动 Dashboard 和后端完整 workflow：

```bash
docker compose \
  -f compose.yaml \
  -f compose.backend.yaml \
  --profile backend \
  --profile storage \
  --profile processing \
  --profile dashboard \
  up --build --wait
```

Dashboard 默认访问 `http://localhost:13000/dashboard`。它在容器内使用 `http://analytics-api:4002`，不需要数据库环境变量。

在 Protocol Consolidation 完成前，Phase 6 的 Protocol V2 仍受 site-level feature flag 保护。使用上面的手动 Compose 命令后，本地 `site_playground` 需要额外启用一次：

```bash
docker compose \
  -f compose.yaml \
  -f compose.backend.yaml \
  --profile backend \
  --profile storage \
  --profile processing \
  --profile dashboard \
  exec -T postgres \
  psql -U analytics -d analytics -c \
  "INSERT INTO analytics_feature_flags (site_id, protocol_v2_enabled, analytics_enabled)
   VALUES ('site_playground', TRUE, TRUE)
   ON CONFLICT (site_id) DO UPDATE
   SET protocol_v2_enabled = TRUE, analytics_enabled = TRUE;"
```

这是开发阶段迁移开关，不是最终用户配置。Protocol Consolidation 完成后，该步骤和 `protocol_v2_enabled` 都应删除；新的用户配置只面向观测能力。

运行 Dashboard 浏览器 E2E 前安装 Chromium：

```bash
pnpm playwright:install
pnpm e2e:dashboard
```

## Check

```bash
pnpm check
```

执行 Prettier 格式检查、TypeScript 类型检查和 ESLint。

## Format

```bash
pnpm format
```

使用 Prettier 格式化项目文件。格式规则包括统一缩进、引号、分号、trailing comma 和文件末尾 newline。

Import 顺序由 ESLint `import/order` 检查，代码边界的空行由 ESLint padding 规则检查。新增 workspace 时，应提供同名的 `format` 和 `format:check` scripts，使根目录命令自动覆盖它。

## Test

```bash
pnpm test
```

验证 Phase 3 Storage、Processor 和 Analytics API contract fixtures：

```bash
pnpm analytics:contract:validate
```

该命令校验 OpenAPI 3.1 JSON contract、canonical fixtures、Raw Event 语义、UTC 聚合结果和 API 响应结构。`/overview` 查询站点累计 Page Views；Reports API 使用 `/reports/{from}/{to}/...` 路径并限制为最多 366 天。PR1 提供 contract 和 fixtures；PR2 另外提供 PostgreSQL Storage 和 migration。

验证 Phase 2 HTTP contract fixtures：

```bash
pnpm http:validate
```

该命令检查 request/response fixture 的结构、状态码、header 格式、scenario ID、配置引用、边界 payload、CORS response 和 setup 状态，不启动 Collector。

命令会执行 Protocol 校验以及所有已创建 package 的单元测试。

`pnpm test` 同时校验 Event Protocol V1 的合法和非法 fixtures。

## PostgreSQL Storage

启动 Phase 3 PostgreSQL：

```bash
cp .env.example .env
docker compose --profile storage up -d --wait postgres
export DATABASE_URL=postgres://analytics:analytics@localhost:5432/analytics
pnpm db:migrate
```

运行 PostgreSQL 集成测试：

```bash
DATABASE_URL=postgres://analytics:analytics@localhost:5432/analytics pnpm test:integration
```

PostgreSQL migrations are owned by the repository infrastructure and run by
the standalone `db-migrator`. Collector, Processor, and Analytics API do not
create or upgrade the schema during startup. Deploy the migration job before
deploying services that require the schema.

启动 Collector 和 PostgreSQL 的开发 workflow：

```bash
pnpm docker:backend
```

该命令同时启用 `backend` 和 `storage` profiles。Collector 使用 PostgreSQL；没有 `DATABASE_URL` 时不会静默回退到 InMemory Sink。
`pnpm docker:backend` 会先等待 PostgreSQL 健康、执行 migration，再启动 Collector 和 Playground。

启动 Processor 和 Analytics API workflow：

```bash
pnpm docker:processing
```

该命令启用 `backend`、`storage` 和 `processing` profiles，先执行 migration，再启动 Collector、Processor、Analytics API 和 Playground。Analytics API 地址为 `http://localhost:4002`。

运行完整 Phase 3 E2E workflow：

```bash
pnpm e2e:analytics
```

该命令会启动独立的 PostgreSQL、Collector、Processor one-shot 和 Analytics API 测试环境，逐个执行 canonical fixtures，结束后自动清理自己的容器和 volume，不影响用户已有 PostgreSQL volume。

单独运行 Protocol 校验：

```bash
pnpm protocol:validate
```

## Build

```bash
pnpm build
```

构建当前所有 TypeScript packages，再构建 Next.js Playground 的生产版本。

## Docker Development

需要 Docker 和 Docker Compose v2：

```bash
pnpm docker:dev
```

该命令会构建并启动 Router Playground，访问地址仍为：

```text
http://localhost:3000
```

默认 Compose 只运行 Playground，不包含 PostgreSQL；页面内的 SDK workflow 使用本地 MockTransport，不发起真实 API request。通过 backend override 可以额外启动 Phase 2 Collector。

Playground 默认使用 MockTransport。要启用本地 Collector workflow，在 `.env` 中设置：

```env
NEXT_PUBLIC_ANALYTICS_TRANSPORT=fetch
NEXT_PUBLIC_ANALYTICS_ENDPOINT=http://localhost:4001/v1/events
NEXT_PUBLIC_ANALYTICS_INGEST_KEY=public-key-example
NEXT_PUBLIC_ANALYTICS_SITE_ID=site_example
```

`NEXT_PUBLIC_ANALYTICS_ENDPOINT` 必须是完整的 `POST /v1/events` URL，不能只填写 Collector base URL。浏览器会自动发送 `Origin`，Collector 会执行 CORS、Origin、Ingest Key、Schema 和限流校验。

启动 Phase 2 Collector：

```bash
docker compose --profile backend up --build collector
```

如果同时启动 Playground，并要求它等待 Collector 健康后再启动：

```bash
docker compose \
  -f compose.yaml \
  -f compose.backend.yaml \
  --profile backend up --build
```

也可以直接运行：

```bash
pnpm docker:backend
```

默认的 `pnpm docker:dev` 不加载 backend override，仍然只启动使用 MockTransport 的 Playground。

Collector 默认监听 `http://localhost:4001`，健康检查地址为：

```text
http://localhost:4001/health
```

当前 Collector 提供健康检查、配置加载、`POST /v1/events`、Origin/CORS、Public Ingest Key 校验和单进程限流。合法事件暂存于进程内 InMemory Sink；Origin 必须配置在对应 site/environment 的 `allowed_origins` 中。

生成本地测试 key（命令只输出 key，不修改 TOML）：

```bash
cargo run -p collector -- key generate --site site_example --environment production
```

发送一个合法 EventBatch：

```bash
curl -i \\
  -X POST http://localhost:4001/v1/events \\
  -H 'Content-Type: application/json' \\
  -H 'X-Ingest-Key: <configured-key>' \\
  -H 'Origin: https://www.example.com' \\
  --data '{"schema_version":1,"events":[{"schema_version":1,"event_id":"01J00000000000000000000000","type":"page_view","site_id":"site_example","occurred_at":1760000000000,"path":"/about"}]}'
```

成功请求返回 `202` 和已接收事件数量。每个 `site_id + Origin` 默认每分钟允许 600 个请求，超限返回 `429`。当前 Collector 不持久化事件，重启后 InMemory Sink 会清空。

启用完整 Compose workflow：

```bash
docker compose \
  -f compose.yaml \
  -f compose.backend.yaml \
  --profile backend up --build
```

Transport 不自动重试，也不持久化发送失败的事件。Collector 的 `400`、`401`、`403`、`413`、`429` 和 `5xx` 响应会转换为可识别的 `FetchTransportError`。

## CI

GitHub Actions 会复用本地检查命令，并额外验证 Docker Compose 配置。CI 不构建或启动 Docker 镜像。

Collector 配置文件路径可以通过 `COLLECTOR_CONFIG` 指定；示例值见 `.env.example`。

## 当前范围

当前已包含：

- `observer-core` 的通用导航契约
- `observer-next` 的 Next.js App Router Adapter
- `analytics-core` 的基础事件管线
- `analytics-browser` 的 Browser SDK runtime 和 Context provider
- Browser SDK 的本地 Mock Buffer workflow
- `@web-analytics/transport` FetchTransport 和 Collector API workflow
- Next.js Router Playground

当前仍不包含：

- Buffer 持久化、离线队列和 BeaconTransport
- Analytics API 和 Dashboard
