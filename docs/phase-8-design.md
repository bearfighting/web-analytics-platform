# Phase 8 Design — MVP 用户配置与能力管理

> Status: Planned
> Scope: capability configuration、站点接入配置、Dashboard 管理和运行时生效语义

## 1. 阶段目标

Phase 8 把 Phase 7 的内部 capability contract 转化为用户可以理解和管理的产品配置。用户配置功能，不配置 Protocol、schema、generation 或 parser 实现细节。

本阶段只支持单部署管理员边界。配置 API 需要受部署级 admin credential 保护，但不实现组织、成员、角色或细粒度权限。

Admin credential 由部署级 secret 或环境变量 bootstrap，不通过公开 Dashboard 创建。配置 API 和公开事件接收 API 使用不同的认证边界；credential 支持轮换和撤销，不能写入浏览器 bundle、事件 payload 或普通业务日志。

## 2. PR0 — Pre-configuration Hardening

Phase 8 正式实现用户配置前，先集中处理前序 Phase review、代码审查和干净环境验证发现的、尚未由 Phase 7 PR2.1 处理的已确认问题。PR0 不重复处理 protocol namespace consolidation，不引入 capability 配置，不新增用户功能，也不改变已确认的 Page View、Visitor、Session、Dimensions 和 Dashboard 语义。

### 2.1 收集范围

PR0 只收录有证据的问题：

- 真实运行时 bug；
- CI、Docker、构建、路径和部署问题；
- 跨工作目录、干净 checkout 或不同运行环境下的失败；
- 测试覆盖不足导致的明确回归风险；
- 文档与实际行为不一致；
- 已确认的架构边界问题。

以下内容不属于 PR0：

- 新产品能力或新 capability；
- Custom Events、Web Vitals、Conversion/Funnel 等功能实现；
- 只有风格偏好的重构；
- 没有复现依据的猜测；
- Phase 8 配置模型本身的新需求。

### 2.2 Bug Register

所有候选问题必须在实现前登记，并包含 ID、Area、Severity、Evidence、Impact、Fix scope、Regression test 和 Status。Severity 使用 P0、P1、P2；Status 使用 `Open`、`Fixed`、`Verified` 或 `Deferred`。Phase 7 PR2.1 已登记的 protocol namespace、canonical path 和 fixture path 问题不再复制到本表。

当前没有已登记的 Phase 8 专属问题。后续发现的问题追加到本表；`services/processor/src/capabilities.rs` 中的 canonical capability manifest 继续使用 `include_str!` 编译期嵌入，不属于运行时路径 bug。Phase 8 的用户 capability 配置也不能替换这份静态 contract；用户配置和静态 contract 必须保持分离。

后续检查发现的问题追加到同一张表，不在实现过程中隐式扩大 PR0 范围。无法在本 PR 处理的问题必须记录延期原因和后续归属。

### 2.3 PR0 执行流程

```text
代码和环境审查
  → 登记并分级问题
  → 冻结 PR0 scope
  → 修复并增加回归测试
  → 全量验证
  → 关闭或明确延期
  → 开始 Phase 8 配置实现
```

### 2.4 PR0 完成条件

- 所有 P0/P1 问题关闭；
- 影响 CI、Docker、干净环境和跨工作目录执行的问题已修复；
- 关键 P2 问题已处理，或有明确的 `Deferred` 原因和后续 PR；
- 每个修复都有对应回归测试或可重复的验证步骤；
- `pnpm check`、`pnpm test`、integration test 和相关 E2E 通过；
- 不修改既有 SQL migration；
- 不改变现有 Page View、Visitor、Session 和 Dimensions 语义；
- 不把临时修复变成新的用户配置或内部 rollout flag；
- 架构边界发生变化时新增 ADR。

## 3. 配置模型

建议的逻辑模型：

```text
site
  id
  name
  environment
  enabled

site_capabilities
  site_id
  capability
  enabled
  settings
  updated_at
  version

site_ingest_policies
  site_id
  allowed_origins
  public_ingest_keys
  rate_limit_policy
  updated_at
  version
```

正式字段、索引和约束必须在实现前通过 migration design 和 ADR 冻结。

## 4. 配置边界

### 用户可以配置

- Page Views、Browser Context、Visitors、Sessions、Dimensions；
- Custom Events 和 Web Vitals 的启用状态及必要设置；
- Conversion 和 Funnel 定义；这些定义依赖 Phase 7 已冻结的事件 contract，但由 Phase 8 提供用户管理入口；
- Geo country 的启用状态；region/city 只有在 Geo PR2 纳入本次 MVP release 后才允许配置精度级别；
- Origin allowlist；
- Ingest Key 的创建、轮换和撤销；
- consent 和隐私选项。

### 用户不能配置

- Protocol 版本；
- schema version；
- Processor generation；
- parser version；
- 内部 rollout flag；
- 数据库表名和 migration 版本；
- SDK 内部 buffer 或 retry 实现。

## 5. 依赖和保存校验

配置 API 保存时必须拒绝：

- 未知 capability；
- 非法 settings；
- 缺少依赖的 capability；
- 不兼容的 Origin、environment 和 identity 配置；
- 空的或重复的 Origin；
- 不符合策略的隐私设置；
- 缺少部署级 admin credential 的修改请求。

依赖示例：

```text
sessions       → anonymous_visitors
dimensions     → browser_context
conversions    → custom_events
funnels        → conversions / custom_events
```

## 6. 配置 API

Analytics API 或独立受保护的 configuration namespace 必须提供：

- 查询站点和 capability 状态；
- 更新 capability；
- 查询和更新 ingest policy；
- 创建、轮换和撤销 Ingest Key；
- 创建和修改 Conversion/Funnel 定义；
- 返回校验错误、版本冲突和生效状态；
- 配置审计记录。

Dashboard 只调用配置 API，不直接访问 PostgreSQL。

## 7. 生效语义

实现前必须通过 ADR 冻结以下规则：

- 配置是 site 级还是 environment 级；
- `version` 如何递增和检测并发更新；
- Collector、Processor 和 API 的读取缓存及刷新间隔；
- 配置服务暂时不可用时使用最后已知配置还是拒绝请求；
- 已发送事件和新配置之间的边界；
- capability 关闭后历史数据是否仍可查询；
- capability 重新开启是否需要 backfill；
- 配置回滚如何影响已经生成的事实；
- Conversion/Funnel 定义变化是否触发历史重算；
- Geo parser 或 dataset 变化是否生成新版本。

默认行为是保留最后一次有效配置，并让配置变更只影响生效时间之后的新采集和处理行为；任何例外必须在 ADR 中说明。

## 8. Dashboard

Dashboard 至少提供：

- capability 状态总览；
- 依赖和校验提示；
- 保存、冲突和失败状态；
- 配置生效状态和最后更新时间；
- Origin 和 Ingest Key 管理；
- Conversion/Funnel 定义管理；
- consent 和隐私说明。

Dashboard 不展示 Protocol 版本、schema、generation 或 parser rollout 信息。

## 9. 测试计划

- 配置 schema 和 migration；
- admin credential 和未授权请求；
- 未知 capability、非法 settings 和依赖错误；
- capability enable/disable；
- 配置版本冲突；
- Collector、Processor、API 读取配置；
- 配置服务不可用时的 fallback；
- Origin 和 Ingest Key 轮换；
- Dashboard 保存、刷新和错误状态；
- 配置变更后的完整 E2E；
- 历史数据可查询和 backfill 边界。

## 10. Phase 8 退出条件

- 配置模型和 migration 已冻结；
- capability、ingest policy、consent 和 authorization 边界清晰；
- authorization 限定为单部署管理员，不引入组织或角色模型；
- 配置 API 和 Dashboard 已完成；
- Collector、Processor 和 Analytics API 使用同一份配置语义；
- 配置失败、回滚、旧配置和历史数据行为有测试；
- 用户无需理解内部协议版本或处理器实现细节；
- 完成至少一条配置变更后的端到端 workflow。
