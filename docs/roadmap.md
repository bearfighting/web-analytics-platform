# Web Analytics Platform Roadmap

## 项目策略

项目按线性方式推进：一次只实施一个主要模块，完成并验证后再添加下一个模块。模块之间通过明确的契约衔接，不提前创建没有实际内容的长期空 package。

整个 MVP 的成功标准是：一个 Next.js App Router 网站接入 SDK 后，可以在 Dashboard 看到基础浏览统计。

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

当前状态：PR1 Contract、PR2 PostgreSQL Raw Event Storage 和 PR3 Processor 已完成，下一步进入 PR4 Analytics API。

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

## Phase 5 — Analytics Semantics and Identity Design

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

目标：根据 Phase 5 已确认的语义，实现浏览器上下文、匿名 Visitor、Session 和对应查询能力。

交付：

- 按已确认契约生成和持久化匿名 Visitor ID。
- Sessionization 和对应的 Processor 聚合。
- Referrer、UTM、Language / timezone、Device、Browser、OS。
- 对应 Raw Event / Aggregate migration。
- Analytics API 和 Dashboard 维度、筛选及空数据状态。
- Visitor、Session 和维度的端到端 fixtures。

Country / IP 不属于本阶段必须内容。

## Phase 7 — Stabilization

目标：在不扩大产品范围的前提下提高可用性。

交付：

- 浏览器兼容性测试
- SDK Bundle size 检查
- Collector 错误处理
- PostgreSQL migration 测试
- End-to-end regression fixtures
- 基础 retention
- 部署文档
- npm SDK 发布流程

## 后续方向

只有出现真实需求后再考虑：

- React Router / TanStack Router Adapter
- Custom Events
- Web Vitals
- Error Analytics
- Conversion / Funnel
- 更复杂的 Geo
- Realtime
- ClickHouse / Kafka
- 多组织和复杂权限
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
