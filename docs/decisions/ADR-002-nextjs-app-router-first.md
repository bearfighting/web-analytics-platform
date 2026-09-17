# ADR-002：首期只支持 Next.js App Router

- Status: Accepted
- Date: 2026-09-17

## Context

项目第一阶段的目标是尽快打通浏览器端网页浏览统计的核心 workflow，同时为未来支持不同 Router strategy 留出扩展空间。

## Decision

首期只实现 Next.js App Router。Router 行为通过通用 NavigationObserver 契约表达，具体 Router 集成通过 Adapter 实现。Phase 0 的 Playground 只观察 Next.js 导航，不承担正式 Analytics 语义。

首期覆盖 initial、pathname、search params、push、replace、back / forward 和 hash 行为。Hash 先记录为 Router 行为，不默认计为 Page View。

React Router、TanStack Router 和其他 Adapter 延后到出现真实需求时新增，不修改 Analytics Core 的通用接口。

## Consequences

- 可以用一个真实且简单的 Next.js 项目验证导航行为。
- Client SDK 和 Observer 的边界必须保持 framework-agnostic。
- 其他 Router 的具体差异不能通过修改 Next.js 专用代码来解决。
- Playground 的调试日志不等同于最终 Analytics Event。
