# Web Analytics Platform

一个 framework-agnostic、privacy-first、self-host friendly 的 Web Analytics Platform。

项目第一阶段聚焦于：

> 为 Next.js App Router 网站提供浏览器端 Page View 和基础网站使用统计。

## 当前状态

项目已完成 Phase 0、Phase 1、Phase 2，以及 Phase 3 PR1 Contract、PR2 PostgreSQL Raw Event Storage、PR3 Page View Processor、PR4 Analytics API 和 PR5 端到端 Workflow，并完成 Phase 4 PR1 Dashboard App Skeleton、PR2 Analytics API Query Client、PR3 Overview Dashboard 和 PR4 Timeline/Top Pages。

当前已具备：

- Monorepo 基础结构
- Event Protocol V1 Schema、examples、fixtures 和自动校验
- Next.js App Router Router Playground
- `observer-core`、`observer-next` 和 `analytics-core` 的基础实现
- `analytics-browser` 的事件 runtime、Browser Context 和可注入 Transport contract
- `analytics-browser` 的有界内存 Buffer、定时 flush 和本地 Mock workflow
- `@web-analytics/transport` 的 FetchTransport 和 Collector 错误映射
- Docker / Docker Compose 开发环境
- Rust Collector foundation、TOML 配置加载和 `/health` 健康检查
- `POST /v1/events`、Event Protocol V1 校验、Origin/CORS、Public Ingest Key、单进程限流和 InMemory Sink
- PostgreSQL `raw_events` migration、幂等 Raw Event Sink 和 storage Compose profile
- Page View Processor、daily/routes/totals 聚合和 processing Compose profile
- Analytics API 的 Overview、Reports、Timeline、Top Pages 和 processing Compose profile

Phase 0 的 Event Protocol、Router Playground、Docker 开发环境和基础工程治理已经完成。可以参考 [Getting Started](docs/getting-started.md) 启动项目。

Storage、Processor、Analytics API 和 Dashboard 按照路线图线性实现。Playground 默认使用 MockTransport；显式配置后可以向本地 Collector 发起真实 API request。

## 目标 Workflow

```text
Next.js Website
  → Browser SDK
  → Next.js Navigation Observer
  → Event Protocol
  → Collector
  → PostgreSQL
  → Processor
  → Analytics API
  → Dashboard
```

## MVP 范围

项目最终聚焦于浏览器端可以获得的网页浏览信息，并按 roadmap 分阶段交付：

```text
Phase 3  Page Views、Pages / Paths
Phase 5  Visitor、Session 和浏览器维度语义设计
Phase 6  Referrer、UTM、Device、Browser、OS 等实现
```

当前已实现的浏览器 SDK 只负责 Page View 事件和基础 Browser Context；Visitor、Session 和维度统计不在当前 Phase 3 实现。

第一阶段只实现 Next.js App Router Adapter，同时通过通用 `NavigationObserver` 接口为未来支持其他 Router 留出空间。

暂不实现：

- 服务端事件和 Server SDK
- React Router、TanStack Router 等其他 Adapter
- Conversion、Funnels、Replay、Heatmap 和 A/B Testing
- 复杂 Geo、IP 持久化和指纹识别
- Kafka、ClickHouse 和复杂实时流处理

## 开发顺序

```text
Phase 0  Monorepo、Protocol、Router Playground、Docker
Phase 1  Client SDK 和 Next.js Adapter
Phase 2  Backend Collector
Phase 3  Storage、Processor、Analytics API
Phase 4  Dashboard
Phase 5  Analytics Semantics 和 Identity Design
Phase 6  Browser 和 Analytics Dimensions
Phase 7  Stabilization
```

## 文档

详细设计文档位于 [docs/](docs/README.md)：

- [Architecture Design](docs/architecture-design.md)
- [Roadmap](docs/roadmap.md)
- [Monorepo Design](docs/monorepo-design.md)
- [Phase 0 Design](docs/phase-0-design.md)
- [Event Protocol](docs/event-protocol.md)
- [Router Playground](docs/router-playground.md)
- [Phase 1 Design](docs/phase-1-design.md)
- [Phase 2 Design](docs/phase-2-design.md)
- [Phase 3 Design](docs/phase-3-design.md)
- [Phase 4 Design](docs/phase-4-design.md)
- [Analytics API OpenAPI Contract](docs/analytics-api.openapi.json)

项目协作规则见 [AGENTS.md](AGENTS.md)。CI 使用与本地相同的统一脚本，并额外验证 Docker Compose 配置。

Phase 3 PR1 contract 可以通过以下命令验证：

```bash
pnpm analytics:contract:validate
```

Phase 3 API contract 区分无日期的站点累计 Overview，以及必须提供 `from/to` 的 Reports API；详见 [Analytics API OpenAPI Contract](docs/analytics-api.openapi.json)。

启动完整的 Phase 3 processing workflow：

```bash
pnpm docker:processing
```

Analytics API 在 `http://localhost:4002` 提供查询接口；Processor 处理 Raw Events 后可查询 Overview、Timeline 和 Top Pages。

验证完整的 Phase 3 链路：

```bash
pnpm e2e:analytics
```

该命令使用独立 Compose project 和测试端口，不删除现有 PostgreSQL volume。

Phase 3 PostgreSQL Storage 需要先启动 storage profile 并执行 migration：

```bash
docker compose --profile storage up -d --wait postgres
export DATABASE_URL=postgres://analytics:analytics@localhost:5432/analytics
pnpm db:migrate
DATABASE_URL=postgres://analytics:analytics@localhost:5432/analytics pnpm test:integration
```

## 技术方向

```text
Client / Dashboard   TypeScript、React、Next.js、pnpm
Backend              Rust（Backend 阶段引入）
Storage              PostgreSQL（Storage 阶段引入）
Development          Docker、Docker Compose
Protocol             JSON Schema、JSON / HTTP
```

## 设计原则

- Client SDK 不依赖具体 Framework 或 Router。
- Router Adapter 只负责观察导航。
- Client 与 Backend 通过版本化 Event Protocol 通信。
- Dashboard 不直接访问数据库。
- 模块按阶段、线性创建。
- 优先完成可运行的核心 Workflow，再逐步增加复杂能力。

## License

本项目使用 [MIT License](LICENSE)。
