# Getting Started

## Requirements

- Node.js 22 LTS
- pnpm 11
- Rust 1.96.0 with Cargo

Phase 0 和 Phase 1 不需要 Rust、Cargo、PostgreSQL 或其他后端依赖。Phase 2 Collector 需要 Rust；Docker 是可选的开发方式。

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

验证 Phase 2 HTTP contract fixtures：

```bash
pnpm http:validate
```

该命令检查 request/response fixture 的结构、状态码、header 格式、scenario ID、配置引用、边界 payload、CORS response 和 setup 状态，不启动 Collector。

命令会执行 Protocol 校验以及所有已创建 package 的单元测试。

`pnpm test` 同时校验 Event Protocol V1 的合法和非法 fixtures。

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
- PostgreSQL Storage、Processor、Analytics API 和 Dashboard
