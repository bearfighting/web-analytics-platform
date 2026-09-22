# Web Analytics Platform Roadmap

## 项目策略

项目按线性方式推进：一次只实施一个主要模块，完成并验证后再添加下一个模块。模块之间通过明确的契约衔接，不提前创建没有实际内容的长期空 package。

MVP 的最低成功标准是：一个 Next.js App Router 网站接入 SDK 后，可以在 Dashboard 看到基础浏览统计。当前规划的完整 MVP 范围还包括 Phase 7 的稳定化和 Phase 8 的产品能力。

## Phase 0 — Project Foundation

详细执行方案见：[phase-0-design.md](phase-0-design.md)。

目标：建立 Monorepo、开发环境、Router Playground 和 Event Protocol V1，并完成基础工程治理。

当前状态：已完成。

交付：

- Monorepo 根目录和 workspace 配置
- `AGENTS.md`
- TypeScript / pnpm 基础配置
- `protocol/` 目录
- Event Protocol V1 初稿
- Protocol examples 和最小 validation fixture
- Next.js App Router Router Playground
- Docker / Docker Compose 开发环境
- `.env.example`、`.dockerignore` 和 Node / pnpm 版本锁定文件
- 最小的统一脚本入口结构
- GitHub Actions CI 和 Phase 0 架构决策记录

本阶段不创建或实现 Client SDK、Backend、Storage、Processor、API 或 Dashboard 模块。Router Playground 只是测试目标和行为参考，不包含 Analytics 实现。Compose 只提供基础开发环境，不提前容器化尚未存在的业务服务。

Router Playground 至少覆盖：

- 首次页面加载
- Next `<Link>` push navigation
- `router.push()`
- `router.replace()`
- 浏览器 back / forward
- pathname 变化
- search params 变化
- hash 变化的观察决策
- 动态路由页面
- 嵌套路由和共享 layout

Rust / Cargo workspace 不属于 Phase 0，等进入 Backend 阶段时再建立。PostgreSQL profile 同样延后到 Storage 阶段。验收：Monorepo 可以通过统一命令启动和验证基础开发环境；Protocol Schema、examples 和 fixture 可以自动校验；Router Playground 可以独立启动并复现常见导航场景，后续 Client SDK 可以直接接入测试。

## Phase 1 — Client SDK

详细实施方案见：[phase-1-design.md](phase-1-design.md)。

目标：实现第一个可使用的浏览器端 SDK，并建立可扩展的 Router Adapter 机制。

当前状态：已完成。SDK 已支持 Next.js App Router、Browser Context、有界内存 Buffer、flush 和本地 MockTransport workflow。

交付：

- `NavigationObserver` 接口
- `NavigationEvent` 类型
- `AnalyticsEvent` / `PageViewEvent` 类型
- `Transport` 接口
- `createAnalytics()`
- `pageview()`
- `observe()`
- 基础 `beforeSend()` 扩展点
- 基础事件 ID、有界内存 Buffer 和 flush
- `observer-next`，只支持 Next.js App Router
- 基础 Browser Context 采集
- 本地 MockTransport workflow

不实现 React Router、TanStack Router 或其他 Adapter。其他 Router 只通过接口设计保留扩展空间。

真实 Fetch/Beacon Transport、API endpoint 和 Backend 契约留待 Phase 2 确定后实现。

## Phase 2 — Backend Collector

详细实施方案见：[phase-2-design.md](phase-2-design.md)。

目标：创建 Backend 模块，接收 Client SDK 发送的 Event Protocol V1 事件。

交付：

- `POST /v1/events`
- Event Schema 校验
- site / environment 校验
- Origin allowlist
- Public Ingest Key
- 基础 payload / batch 限制
- 简单限流
- 内存或开发用接收适配器
- 健康检查、结构化日志

当前状态：已完成。已实现 Rust Collector、HTTP ingestion、Event Protocol V1 校验、Origin/CORS、Public Ingest Key、单进程限流、InMemory Sink 和 FetchTransport workflow。

本阶段暂不要求正式数据库存储；Backend 先能正确接收、校验和观测事件。

验收：SDK 可以向 Collector 发送事件，Collector 能返回明确的成功和错误响应。

## Phase 3 — Storage and Processing

详细执行方案见：[phase-3-design.md](phase-3-design.md)。

目标：添加 Storage 模块，使事件可以持久化并产生基础统计结果。

当前状态：Phase 3 PR1–PR5 已完成，下一步进入 Phase 4 Dashboard。

交付：

- PostgreSQL Adapter
- `raw_events` 表和 migration
- `page_view_totals` 累计聚合
- 幂等 Page View Processor
- `page_view_daily` 和 `page_view_routes` 聚合
- Timeline 和 Top Pages 查询
- Analytics API 的最小实现
- Client → Collector → PostgreSQL → Processor → Analytics API E2E workflow

本阶段不实现 Visitor、Session、Browser Dimensions 或复杂实时处理；这些能力需要先完成独立的数据契约和 SDK 语义设计。

验收：Client SDK → Backend → PostgreSQL → Processor → Analytics API 完整链路可运行，重复事件不会造成重复 Page View 统计。

## Phase 4 — Dashboard

详细实施方案见：[phase-4-design.md](phase-4-design.md)。

当前状态：Phase 4 PR1–PR5 Dashboard workflow 已完成。

目标：添加 Dashboard 模块，让项目使用者可以直观看到网站使用情况。

交付：

- Overview 页面
- Page Views 总数
- Timeline
- Top Pages
- 日期范围
- Loading、Empty、Error 状态
- Site 选择的基础结构

Dashboard 不实现统计逻辑，只消费 Analytics API。

Phase 4 退出前通过 `pnpm e2e:dashboard` 验证 Collector → PostgreSQL → Processor → Analytics API → Dashboard 的完整流程。

## Phase 5 — Analytics Semantics and Identity Design

详细设计方案见：[phase-5-design.md](phase-5-design.md)。

当前状态：Phase 5 设计已完成。

目标：在实现 Visitor、Session 和浏览器维度之前，先完成指标语义、匿名身份和数据生命周期设计。

交付：

- Visitor 的定义、匿名 ID 生命周期和存储边界。
- Session 的 inactivity timeout、跨午夜、跨标签页和异常场景语义。
- `occurred_at`、`received_at`、迟到事件和重处理规则。
- Browser Context 字段的稳定 schema、版本和隐私边界。
- Referrer、UTM、Language、timezone、Device、Browser、OS 的指标定义。
- API 和 Dashboard 需要支持的维度、过滤器和空数据语义。
- 相关 ADR、Protocol/SDK 变更设计、canonical fixtures 和迁移计划。

本阶段以研究、契约和可验证 fixture 为主；没有稳定契约前，不实现正式 Visitor、Session 或维度聚合。

Country / IP、指纹识别和跨设备识别不属于本阶段设计目标。

## Phase 6 — Browser and Analytics Dimensions

详细实施方案见：[phase-6-design.md](phase-6-design.md)。

目标：根据 Phase 5 已确认的语义，实现浏览器上下文、匿名 Visitor、Session 和对应查询能力。

当前状态：Phase 6 PR1、PR2、PR3、PR4、PR5 实现和 E2E 验收已完成。

交付：

- 按已确认契约生成和持久化匿名 Visitor ID。
- Sessionization 和对应的 Processor 聚合。
- Referrer、UTM、Language / timezone、Device、Browser、OS。
- 对应 Raw Event / Aggregate migration。
- Analytics API 和 Dashboard 维度、筛选及空数据状态。
- Visitor、Session 和维度的端到端 fixtures。

Country / IP 不属于本阶段必须内容。

## Phase 7 — Stabilization

详细执行方案见：[phase-7-design.md](phase-7-design.md)。

目标：在不扩大产品范围的前提下完成协议收敛、稳定性验证和第一个 release candidate 准备。

交付：

- 浏览器兼容性测试
- SDK Bundle size 检查
- Collector 错误处理
- PostgreSQL migration 测试
- End-to-end regression fixtures
- 基础 retention
- 部署文档
- npm SDK 发布流程

实施顺序和验收标准以 Phase 7 设计文档为准。Protocol consolidation 是发布前置步骤；retention 在策略批准前只允许 dry-run，不启用自动删除。

## Phase 8 — MVP Product Completion

详细执行方案待补充，开始实施前新增 `docs/phase-8-design.md`。

目标：在 Phase 7 稳定化基础上补齐 MVP 所需的产品能力，但不把高隐私风险和高数据量能力混入稳定化阶段。

Phase 8 的完成与 Phase 7 一起构成 MVP 发布范围：

```text
Phase 6 Analytics workflow
  → Phase 7 Stabilization and release readiness
  → Phase 8 MVP Product Completion
```

建议交付顺序：

- Router Adapter 扩展：React Router、TanStack Router 等具体 Adapter。
- Custom Events：协议、Collector 接收、Raw Event 保存、Processor 处理和 API 查询契约。
- Web Vitals：浏览器采集、指标 schema、聚合语义和 Dashboard/API 展示。
- Visitor、Session、Dimension 语义修订：只有在确认当前 Phase 5/6 语义不足时才修改，并通过新的 ADR、generation 和兼容性测试完成。
- Conversion / Funnel：基于事件的转化定义、漏斗计算和查询 API。
- Geo PR1：基础 Geo 维度，例如 country/country code；不保存原始 IP，使用可版本化的解析结果。
- Geo PR2：在确认基础 Geo 的数据质量和隐私边界后，再增加 region/city 等扩展维度。

Phase 8 每项能力必须独立完成：

```text
Protocol / contract
  → migration and domain model
  → SDK / Collector / Processor
  → API / Dashboard
  → canonical fixtures
  → end-to-end verification
```

Phase 8 不应直接修改 Phase 7 的稳定性目标；如果需要 breaking protocol 或 API 变更，必须先更新设计文档和迁移方案。

Phase 8 实现新能力时，必须同步建立稳定的内部 capability 边界，但暂不要求提供用户可编辑的动态站点配置。用户配置 API、Dashboard 管理、运行时刷新和回滚属于 Phase 8 之后的独立产品配置阶段。

Phase 8 明确保持单体部署边界：继续使用 PostgreSQL、批处理或 one-shot Processor 和现有 HTTP API，不引入消息队列、缓存集群、流处理平台或其他分布式基础设施。Geo PR2 是否实施，取决于 Geo PR1 的数据质量、部署资产和隐私评估，不默认扩大 MVP 范围。

## Phase 9 — Advanced Analytics and Infrastructure

Phase 9 用于需要独立隐私、安全和高数据量设计的高级能力：

- Replay。
- Heatmap。
- 高级 Geo：例如 geospatial polygon、ISP/ASN、VPN/proxy detection 等。
- ClickHouse / Kafka 等专用基础设施。
- 多组织和复杂权限。

Replay 和 Heatmap 默认不属于 MVP。除非出现明确需求，否则不提前创建对应的 Protocol、migration、package 或服务。

Realtime 也不属于当前 MVP 路线。实时推送、持续流式消费、消息队列、缓存集群和分布式聚合只有在出现明确吞吐量或延迟需求后再重新规划。

## 后续专项 — Deployment Modes

当前 Phase 7、Phase 8 和 Phase 9 完成后，再详细设计两种部署模式：

### Single-node Edition

面向个人项目、小型网站、本地或内网部署，目标是通过一个 tarball 和一个命令启动完整功能：

- 一个统一的 analytics server；
- SQLite 文件数据库；
- 内置 Collector、Analytics API 和 Processor worker；
- 内置 Dashboard 静态资源；
- 本地 migration、backup 和 restore 命令；
- 不依赖 PostgreSQL、消息队列、缓存或其他外部基础设施。

Single-node Edition 明确限制为单机、单写入进程和有限吞吐，不承诺水平扩展或多实例共享数据库文件。

### PostgreSQL Edition

面向更完整的部署体验和较高数据量，继续支持独立组件：

```text
db-migrate
  → collector
  → processor
  → analytics-api
  → dashboard
```

PostgreSQL Edition 可以支持多 worker、较高并发和独立运维，但仍不默认引入分布式消息队列、缓存集群或流处理平台。

### Queue 和 Storage 方向

两种部署模式共享 Protocol、领域语义、Processor、API contract、Dashboard 和 canonical fixtures，只替换 Storage、Migration、Queue 和 Runtime packaging：

- SQLite 使用普通 queue table、事务和单 worker；
- PostgreSQL 使用普通 queue table、事务和 `SKIP LOCKED`；
- 不把 pg queue 插件作为强制依赖；
- 将来确有吞吐量需求时，再增加外部 queue adapter；
- SQLite 和 PostgreSQL 使用各自 migration，但共享逻辑 schema contract 和跨数据库 fixture。

实施时预计拆为：

- Deployment PR1：Queue abstraction、SQLite storage 和 SQLite migration；
- Deployment PR2：统一 server runtime、tarball packaging、backup/restore 和 single-node E2E。

该专项暂不进入当前 Phase 7 或 Phase 8 的实现范围。开始实施前新增独立设计文档，例如 `docs/deployment-modes-design.md`，并重新评估 SQLite/PostgreSQL 的功能矩阵、升级路径和数据迁移方案。

## 后续方向

只有出现真实需求后再考虑：

- Error Analytics
- [观测能力模块化与动态配置](feature-modularization-design.md)：Phase 8 在实现新能力时完成内部 capability 边界；Phase 8 之后再实现配置 API、持久化、Dashboard 管理、动态刷新和回滚。用户不配置 Protocol 版本、schema 或内部 rollout flag。
- 模块化实施顺序固定为：统一协议 → 稳定现有 MVP → Phase 8 内部能力模块化 → 配置 API/持久化 → Dashboard 配置 → 动态刷新与回滚。
- 非实时趋势 Summary / Insights API
  - 面向总体趋势和多维度聚合查询。
  - 可以评估 PostgreSQL Materialized View、rollup table 或其他预计算方案。
  - 需要单独定义数据新鲜度、刷新策略、`data_as_of` 语义和维度查询 contract。
  - 不改变当前 `/overview` 与 Reports API 的 Phase 3 contract。

## 阶段推进原则

每个阶段都必须保持以下顺序：

```text
Protocol / contract
  → implementation
  → fixture
  → end-to-end verification
```

优先交付可以运行的纵向切片，不因为未来功能提前引入大型基础设施。

模块创建规则：

- Phase 0 只创建工程骨架、Protocol、Router Playground 和 Docker 开发环境。
- 进入 Client SDK 阶段时，才创建 `packages/` 下的 SDK 和 Observer packages。
- 进入 Backend 阶段时，才创建 `services/collector`。
- 进入 Storage 阶段时，才创建 Rust domain、storage、processor 和 API 实现。
- 进入 Dashboard 阶段时，才创建 `apps/dashboard` 和 query client。
