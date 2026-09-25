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

### 2.5 PR0.5 — Dependencies and Build Tool Refresh

在配置模型和 migration 开始前，升级项目依赖与构建工具。项目以自身构建、测试和部署兼容为目标，不需要为外部消费者维持库的 semver 范围；允许 major 更新，但不得把升级和 capability 配置功能混在同一个 PR。执行日基线见仓库升级记录；具体执行分组如下：

1. **PR0.5a — pnpm 与 Node.js 基线：** 固定 pnpm 12.6.0、Node.js 26.10.0，同步 `packageManager`、engines、`.node-version`、CI、开发文档和 Node Docker 镜像。
2. **PR0.5b — JavaScript 开发与测试工具：** 升级 TypeScript、ESLint、Prettier、Vitest、Playwright 和相关插件，处理配置/API 迁移。若当前插件不支持最新主版本，记录 Deferred 与重试条件，不通过跳过 lint/typecheck 验收。
3. **PR0.5c — 浏览器与应用依赖：** 升级 Next.js/React、Vite、React Router、TanStack Router、`web-vitals` 及 workspace 直接运行依赖；按集成边界拆分并同步 peer ranges。
4. **PR0.5d — Rust 工具链与 crates：** 升级 Rust stable、workspace crates 和 `Cargo.lock`，本地与 Rust Docker 镜像一致，并处理 crate API breaking changes。
5. **PR0.5e — CI 与基础设施镜像：** 升级 GitHub Actions、PostgreSQL 和其他构建期系统依赖。Node 镜像由 PR0.5a 固定，Rust 镜像由 PR0.5d 对齐。

每批 PR 都必须：

- 记录升级前后版本、官方发布来源、breaking changes 和兼容性决定；只使用正式稳定版，不使用 prerelease 或 `latest` 浮动标签。
- 保持锁定版本和声明范围一致。执行 `pnpm install --frozen-lockfile` 不得自动改写 `packageManager` 或 lockfile；Node、pnpm、Rust 和容器版本要在本地、CI、Docker 中对齐。
- 更新后运行对应 package 的 check/test/build；跨边界升级还要运行 migration、Rust integration、Analytics E2E、Dashboard E2E、Router Adapter E2E 和 Compose profile 验证。
- 将不可升级项连同阻塞原因、复现证据和重试条件记录为 Deferred，不为了追求版本号而降低测试或安全边界。

#### 2026-09-24 执行记录

- 已设置 Node.js 26.10.0、pnpm 12.6.0、Rust 1.98.1；PostgreSQL 镜像为 18.6-alpine3.23。Node、Rust、PostgreSQL 与 GitHub Actions 均使用固定版本/发行标签。PostgreSQL 18 更改了数据目录布局，Compose 改用 `/var/lib/postgresql` 并使用新的 `postgres_data_v18` volume；旧 volume 保留，升级数据需要按 Getting Started 的备份/恢复步骤迁移。
- JavaScript workspace 使用 Next.js 16.3.6、React 19.3.0、Vite 8.3.0、Vitest 5.0.1、Prettier 3.9.9、Playwright 1.63.0。Next ESLint 配置迁移到 flat config；Dashboard 测试显式启用 React Vite plugin。
- Rust workspace crates 和 lockfile 已升级；SQLx 0.9 的动态 SQL 安全标记及 Migrator 初始化、jsonschema 0.57 API 变更已迁移。
- **Deferred：TypeScript 7.0.2。** 当前 `typescript-eslint` 8.70.1 声明支持 TypeScript `<6.1.0`；升级 TS7 会越过 parser 支持范围，因此 workspace 暂用 TypeScript 6.0.3。待 typescript-eslint 官方支持 TS7 后重试，并运行完整 typecheck/build。
- **Deferred：ESLint 10.11.0。** 当前 Next.js/React lint 依赖在 ESLint 10 Rule API 上失败（`eslint-plugin-react@7.37.5` 调用已移除的 `context.getFilename`；后续 scope manager API 也不兼容）。workspace 暂用 ESLint/@eslint/js 9.39.5，配合 Next.js flat config；待 Next/React 插件兼容 ESLint 10 后升级，并运行完整 lint/check。
- `pnpm outdated -r` 仅列出上述 TypeScript、ESLint 和 `@eslint/js` 三项；其余 workspace 声明依赖无更新项。`pnpm install --frozen-lockfile`、`pnpm check`、`pnpm test` 和 `pnpm build` 已通过。PostgreSQL 18 migration 与 integration tests 已通过；Analytics E2E 10 个 fixture、Dashboard E2E 全部场景、Router Adapter E2E 三种路由器均通过。所有 Compose profiles 配置校验通过；Dashboard E2E 已从空的容器依赖卷完成 frozen install、镜像构建和完整浏览器流程。首次 Dashboard E2E 重跑因并行集成测试数据库占用默认端口 `15432` 未启动；清理临时数据库后重跑通过。检查期间格式校验发现 Next.js 自动生成的两个 `tsconfig.json` 排版变化，已格式化并由最终 `pnpm check` 复核。

依赖升级实现和主要验证已完成。配置实现开始前仍须通过一次干净 checkout 的 frozen install 与基础构建验收；这项验收只验证当前锁定工具链，不重复完整 E2E。升级中发现的具体 bug 按 PR0 bug register 规则登记，但不因此把用户配置功能混入升级 PR。

### 2.6 Phase 8 顺序执行计划

以下工作按顺序执行；每个 PR 合并并通过本 PR 验收后，才开始下一项。所有 PR 都需包含其边界内的测试与文档，不把失败策略、数据语义或权限决定留给 Dashboard 实现阶段。

| 顺序 | PR                                                       | 范围与出口条件                                                                                                                                                                                                                                                                                                                                                                                   |
| ---- | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 0    | **PR0 — Pre-configuration Hardening**                    | 对照 §2.2 bug register 完成前序问题审查。当前没有已登记的 Phase 8 专属问题；PR0 关闭前须留下本轮审查结论、将候选项登记或说明无证据，并确认 P0/P1 关闭或明确阻塞。不得顺手加入配置功能。                                                                                                                                                                                                          |
| 0.5  | **PR0.5a–e — Dependencies and Build Tool Refresh**       | 实施已完成，版本和验证记录见 §2.5。剩余出口条件是干净 checkout 下 frozen install 与基础构建成功；完成后锁定工具链基线。TypeScript 7 与 ESLint 10 延期不阻止进入配置工作，按记录的重试条件跟进。                                                                                                                                                                                                  |
| 1    | **PR1 — Configuration Semantics and Contracts**          | 新增 ADR 与可校验的配置/API contract。冻结配置主体及 site/environment 关系、旧 `analytics_enabled` 到 capability 默认值的映射、依赖与默认值、关闭后历史查询/重新开启/backfill、并发版本、服务不可用行为、Origin/identity 约束、consent 语义、部署级 admin credential 的 bootstrap 与 Dashboard 调用边界、Ingest Key 轮换/撤销/返回语义，以及配置审计和敏感字段规则。没有数据库 migration 或 UI。 |
| 2    | **PR2 — Configuration Persistence and Migration**        | 按 PR1 冻结的 contract 实现配置表、约束、版本与 additive migration；为已有站点建立确定性默认配置并映射 `analytics_enabled`。旧字段在兼容窗口内保留，迁移重复执行、失败恢复和回滚路径均有测试；本 PR 不切换 Collector/Processor 的运行时读取。                                                                                                                                                    |
| 3    | **PR3 — Protected Configuration API**                    | 实现部署级管理员授权、配置读取/更新、依赖与 settings 校验、版本冲突、审计。API 仅调用配置存储边界，不向浏览器暴露部署级 admin credential；测试覆盖未授权、无效输入、冲突、审计脱敏和持久化读回。                                                                                                                                                                                                 |
| 4    | **PR4 — Collector Ingest Policy Runtime**                | 将站点启用状态、Origin allowlist 和 Ingest Key 策略接入 Collector；实现 PR1 规定的刷新、最后有效配置/拒绝策略、轮换重叠窗口与故障行为。验证启动、配置刷新、Collector 重启、密钥轮换/撤销和已有 TOML 配置迁移边界。                                                                                                                                                                               |
| 5    | **PR5 — Processor and Analytics API Capability Runtime** | 让 Processor 与 Analytics API 按同一份版本化配置语义处理 capability 开关；把 `analytics_enabled` 兼容映射切换到 capability 状态。验证关闭时停止新增相应处理、历史数据按 PR1 规则可查询、重新开启/backfill 与回滚行为明确，并确保站点隔离。                                                                                                                                                       |
| 6    | **PR6 — Dashboard Core Configuration**                   | 在服务端配置 API 可用后实现 capability 与 consent 状态、Origin 管理、Ingest Key 创建/轮换/撤销和最后生效状态。覆盖读取、保存、校验错误、版本冲突、加载/空/错误状态；部署级 admin credential 不进入浏览器 bundle，Ingest Key 明文只按 PR1 规则返回。                                                                                                                                              |
| 7    | **PR7 — Conversion/Funnel Definition Management**        | 独立实现定义的创建/编辑/停用、版本化、校验与 Dashboard 管理。冻结定义变化对已生成 facts、历史查询和 backfill 的影响；避免将业务定义 CRUD 与基础 capability 开关耦合。                                                                                                                                                                                                                            |
| 8    | **PR8 — Configuration End-to-end Acceptance**            | 完成配置变更后的端到端 workflow：受保护写入 → 存储/版本递增 → Collector 与 Processor/API 生效 → Dashboard 展示；覆盖刷新、服务故障、回滚、历史边界和站点隔离。同步更新 Getting Started、运维/备份/恢复说明和 Phase 8 验收记录。MVP 最终浏览器矩阵、retention、发布与 clean Release Candidate 仍由 Release Readiness 负责。                                                                       |

PR1 是模型与 migration 的硬性前置。PR2 先保留兼容路径，直到 PR4/PR5 的运行时切换和回滚验证完成；PR6 依赖 PR3–PR5 已提供且已验证的生效语义；PR7 可在 PR6 后独立实现，但必须先完成 PR1 对定义版本与历史重算的决策。PR8 只做跨层验收与部署文档，不承接未拆分的产品功能。

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
