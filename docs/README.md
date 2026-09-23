# Documentation Index

## 当前文档

- [Architecture Design](architecture-design.md)：长期架构目标和系统边界。
- [Roadmap](roadmap.md)：线性开发阶段和交付顺序。
- [MVP Scope](mvp-scope.md)：MVP 功能范围、排除项、完成定义和发布门槛。
- [Monorepo Design](monorepo-design.md)：仓库结构、模块职责和 Workspace 策略。
- [Phase 0 Design](phase-0-design.md)：项目启动阶段的详细步骤、Checklist 和验收标准。
- [Event Protocol](event-protocol.md)：统一初始 Event Protocol；历史 V1/V2 迁移背景见协议合并设计。
- [Protocol Layout](../protocol/README.md)：事件、Context、服务契约和语义场景的稳定目录约定。
- [Capability Contract](../protocol/capabilities/capabilities.json)：10 个 Analytics capability 的机器可读边界和依赖定义。
- [Protocol Consolidation Refactoring](protocol-consolidation-refactor.md)：上线前合并开发阶段 V1/V2 协议的目标、步骤和验收标准。
- [Feature Modularization Design](feature-modularization-design.md)：协议收敛之后的观测能力模块化、动态配置和实现优先顺序；具体 MVP 阶段以 Phase 7/8 文档为准。
- [Router Playground](router-playground.md)：Next.js、React Router 和 TanStack Router playground 及统一 facade 接入。
- [Phase 1 Design](phase-1-design.md)：Client SDK、Observer 和 Transport 的实施计划。
- [Phase 2 Design](phase-2-design.md)：Backend Collector、HTTP 契约和基础安全控制的历史实施计划。
- [Phase 3 Design](phase-3-design.md)：PostgreSQL Storage、Page View Processor 和 Analytics API 的实施计划。
- [Phase 4 Design](phase-4-design.md)：Dashboard、Analytics API Query Client 和完整 Dashboard workflow 的实施计划。
- [Phase 5 Design](phase-5-design.md)：Analytics Semantics、Visitor、Session、Browser Context 和 Dimensions 的历史契约设计。
- [Phase 6 Design](phase-6-design.md)：Browser Visitor ID、临时 Protocol V2 rollout、Sessionization、Dimensions、Analytics API 和 Dashboard 的历史实施计划。
- [Phase 7 Design](phase-7-design.md)：MVP 功能完善、Protocol consolidation 和 capability 边界。
- [Phase 8 Design](phase-8-design.md)：MVP 用户配置、配置 API 和 Dashboard 能力管理。
- [Release Readiness](release-readiness-design.md)：MVP 最后的完整回归、稳定性、部署和发布验证。
- [Analytics API OpenAPI Contract](analytics-api.openapi.json)：Analytics API v1 的机器可读契约，包含已启用的 Phase 6 reports。
- [Ingest Key Guide](ingest-key.md)：Ingest Key 的生成、配置、Website 使用、Origin 关联和轮换流程。
- [ADR-007：Capability-oriented Configuration](decisions/ADR-007-capability-oriented-configuration.md)：以用户能力而不是内部协议版本提供配置的架构决策。
- [ADR-008：Internal Capability Boundaries](decisions/ADR-008-internal-capability-boundaries.md)：Phase 7 capability contract、依赖和跨层边界。
- PR2 Router Adapters：React Router 7 和 TanStack Router v1 的 NavigationObserver 集成。
- Phase 7 PR2.1：Contract namespace consolidation，收敛协议路径、版本命名和跨语言资源引用。
- Phase 7 PR2.5：统一 Router facade、`RouterAnalyticsBridge` 和参数化 playground / Compose profiles，详见 [Phase 7 Design](phase-7-design.md)。

`AGENTS.md` 位于仓库根目录，作为整个项目的协作和开发规则入口。

## 文档关系

```text
architecture-design.md
        ↓
roadmap.md
        ↓
roadmap.md
        ↓
phase-N-design.md
        ↓
实际代码和测试
```

## 后续文档

后续设计文档统一放在 `docs/` 下：

```text
docs/
├── architecture-design.md
├── roadmap.md
├── monorepo-design.md
├── phase-0-design.md
├── event-protocol.md
├── router-playground.md
├── phase-1-design.md
├── metrics-semantics.md
├── client-sdk.md
├── observers.md
├── testing.md
└── decisions/
```
