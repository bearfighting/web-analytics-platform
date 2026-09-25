# ADR-007：以观测能力而非协议版本提供配置

- Status: Accepted
- Date: 2026-09-22

## Context

开发阶段同时存在 Event Protocol V1 和 V2。V2 feature flag、schema version 和 migration 状态用于内部 rollout 验证，但不应成为最终用户需要理解或修改的产品配置。

项目后续还需要支持 Page Views、Browser Context、Anonymous Visitors、Sessions、Dimensions、Custom Events、Web Vitals 和 Conversion 等能力。如果直接把内部协议开关暴露到 Dashboard，用户配置会被实现细节污染，协议收敛后也会产生不必要的迁移负担。Phase 7 已将当前十项 capability contract 收敛到统一 manifest。

## Decision

1. 在首次正式发布前完成 Protocol V1/V2 consolidation，形成唯一初始 Event Protocol。
2. 删除或停止使用只服务于协议 rollout 的运行时配置，例如 `protocol_v2_enabled`。
3. 后续模块化以用户可理解的观测能力为边界，不以 Protocol 版本、schema、generation 或 parser version 为边界。
4. 用户最终配置 capability，例如 Page Views、Sessions 或 Web Vitals；协议版本、数据迁移、派生 generation 和 parser rollout 属于平台内部实现。
5. 动态配置必须通过服务端持久化配置和权限控制实现。环境变量只作为 bootstrap、部署默认值或紧急控制手段。
6. Capability configuration is site-scoped. Ingest security policy is scoped to a site and environment. Browser consent is a mandatory client-side boundary and cannot be disabled by site configuration. Operator authorization remains a separate deployment-level boundary.

## Consequences

- Phase 7 先完成协议收敛和 capability contract，Phase 8 再实现 Dashboard 动态能力配置。
- 需要先定义 capability contract、依赖、关闭语义、历史数据语义和配置刷新策略，再实现配置 API 与 Dashboard UI。
- `analytics_enabled` 只作为迁移前的过渡字段。PR2 将 true 映射为开启 browser_context、anonymous_visitors、sessions 和 dimensions；false 或缺少旧记录映射为关闭这四项。Page Views 保持开启，其余能力维持当前开启行为。旧字段保留至 PR5 运行时切换及回滚验证完成。
- Dashboard 不显示 V1/V2、schema version、feature flag 或内部 rollout 状态。
- Capability contract 与用户可改状态保持分离；用户配置不能声明、替代或重写 manifest 中的实现边界。

## Sequencing

```text
Protocol consolidation
  → Phase 7 MVP feature completion
  → Phase 8 configuration API and persistence
  → Dashboard capability management
```
