# Documentation Index

## 当前文档

- [Architecture Design](architecture-design.md)：长期架构目标和系统边界。
- [Roadmap](roadmap.md)：线性开发阶段和交付顺序。
- [Monorepo Design](monorepo-design.md)：仓库结构、模块职责和 Workspace 策略。
- [Phase 0 Design](phase-0-design.md)：项目启动阶段的详细步骤、Checklist 和验收标准。
- [Event Protocol](event-protocol.md)：Event Protocol V1 的 Schema、字段和版本策略。
- [Router Playground](router-playground.md)：Next.js App Router 实验场和导航场景。
- [Phase 1 Design](phase-1-design.md)：Client SDK、Observer 和 Transport 的实施计划。
- [Phase 2 Design](phase-2-design.md)：Backend Collector、HTTP 契约和基础安全控制的实施计划。
- [Ingest Key Guide](ingest-key.md)：Ingest Key 的生成、配置、Website 使用、Origin 关联和轮换流程。

`AGENTS.md` 位于仓库根目录，作为整个项目的协作和开发规则入口。

## 文档关系

```text
architecture-design.md
        ↓
roadmap.md
        ↓
phase-0-design.md
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
