# Web Analytics Platform

一个 framework-agnostic、privacy-first、self-host friendly 的 Web Analytics Platform。

项目第一阶段聚焦于：

> 为 Next.js App Router 网站提供浏览器端 Page View 和基础网站使用统计。

## 当前状态

项目目前处于 Phase 0：项目基础设计和工程准备阶段。

当前正在准备：

- Monorepo 基础结构
- Event Protocol V1
- Next.js App Router Router Playground
- Docker / Docker Compose 开发环境

PR2 已加入 Next.js App Router 导航 Playground 和 Docker 开发方式。可以参考 [Getting Started](docs/getting-started.md) 启动 Playground。

正式的 Client SDK、Backend、Storage 和 Dashboard 将按照路线图线性实现，不会在项目启动时一次性创建全部模块。

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

## 第一阶段范围

第一阶段只处理浏览器端可以获得的网页浏览信息：

```text
Page Views
Pages / Paths
Visitors
Sessions
Referrers
UTM
基础 Device / Browser / OS
```

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
```

## 文档

详细设计文档位于 [docs/](docs/README.md)：

- [Architecture Design](docs/architecture-design.md)
- [Roadmap](docs/roadmap.md)
- [Monorepo Design](docs/monorepo-design.md)
- [Phase 0 Design](docs/phase-0-design.md)

项目协作规则见 [AGENTS.md](AGENTS.md)。

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
