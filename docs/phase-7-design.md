# Phase 7 Design — Stabilization and Release Readiness

> Status: Implementation plan
> Scope: Protocol consolidation、CI regression、Browser compatibility、SDK quality、Collector hardening、retention、deployment 和 npm release

## 1. Phase 7 定义

Phase 7 不新增主要产品能力，而是把 Phase 6 的实现收敛为可以稳定验证、部署和发布的第一个 release candidate。

目标 workflow：

```text
Protocol consolidation
  → CI / migration verification
  → Browser and SDK quality
  → Collector runtime hardening
  → Retention and privacy lifecycle
  → Deployment and package release
  → Release candidate verification
```

Phase 7 必须保持现有 Page View、Visitor、Session、Dimension、Analytics API 和 Dashboard 语义不变。除非本设计明确批准，稳定化工作不得借机增加新的统计能力或改变公开 contract。

## 2. 当前基线

### 2.1 已完成或已有基础

- 根目录 `migrations/` 和独立 `tools/db-migrator` 已成为数据库 migration 的 owner。
- `pnpm db:migrate`、PostgreSQL integration test、Analytics E2E 和 Dashboard E2E 已有统一入口。
- Phase 3–6 的主要代码、schema、API contract、fixtures 和生成式数据 workflow 已存在。
- Protocol 目录已按稳定领域划分；开发阶段 V1/V2 合并方案记录在 `docs/protocol-consolidation-refactor.md`。
- CI 已执行依赖安装、类型检查、测试、构建、格式检查和 Dashboard E2E。

### 2.2 Phase 7 仍需完成

- 将 Protocol consolidation 从设计变成一次受控的 breaking refactor，并在发布前冻结最终 protocol。
- 将 PostgreSQL migration、integration test 和 Analytics E2E 纳入标准 CI 验收。
- 定义并自动验证浏览器支持矩阵和 SDK bundle size budget。
- 审计 Collector 的超时、连接池、优雅关闭、错误脱敏、健康检查和失败重试边界。
- 定义安全且可回滚的 Raw Event 与派生数据 retention 生命周期。
- 补齐部署、备份、迁移、回滚、配置和 npm package 发布文档及自动化流程。

## 3. 必须达成

- 干净数据库和已有数据库都可以通过独立 migration runner 升级，且重复执行安全。
- CI 能在真实 PostgreSQL 上执行 migration、integration test、Analytics E2E 和 Dashboard E2E。
- SDK 在声明的浏览器矩阵中通过核心行为测试，产物大小有明确预算并在 CI 中阻止异常增长。
- Collector 在数据库不可用、请求超限、请求超时、优雅关闭和内部错误场景下行为可预测，响应不泄露内部信息。
- retention 删除不会产生孤立事实、错误聚合或无法恢复的半成品；默认不得隐式删除数据。
- 部署顺序固定为：

```text
migration job → schema compatibility check → service deployment → health verification
```

- SDK package 的名称、入口、类型声明、版本、变更记录和发布权限可被独立验证。
- 完整 release candidate checklist 可以在干净环境重复执行。

## 4. 不属于本阶段

- 新 Router Adapter、Custom Events、Web Vitals、Conversion、Funnel、Replay 或 Heatmap。
- 新的 Visitor、Session、Dimension 或 API 统计语义。
- Kafka、ClickHouse、Redis Cluster、Realtime 或多区域部署。
- 真实用户迁移、跨设备身份合并或复杂组织权限。
- 在 retention 策略未获批准前启用自动生产删除。

## 5. 实施原则

### 5.1 先收敛 contract，再发布

开发阶段 V1/V2 protocol 的合并必须在 npm 发布和生产部署前完成。合并时同步更新 schema、TypeScript 类型、Rust 类型、fixtures、Collector、SDK、文档和验证脚本。

### 5.2 数据安全优先

Raw Event 是重建 Page View、Visitor、Session 和 Dimension 的事实源。任何 retention、清理或迁移操作都必须明确影响范围、执行顺序、失败恢复和回滚限制。

### 5.3 稳定化不改变业务语义

Phase 7 的默认目标是发现问题、限制风险和提升可观测性，而不是修改聚合算法。需要改变公开语义时必须另建 ADR 或后续 Phase。

### 5.4 独立基础设施 owner

Migration、部署和 release 工具不属于 Collector、Processor 或 Analytics API 的业务启动流程。服务只消费已经完成兼容性检查的 schema。

## 6. PR 拆分和依赖关系

### PR0 — Protocol consolidation

将开发阶段的 V1/V2 输入收敛为发布前的最终 protocol。具体字段、schema version、flag 清理和兼容策略以 `docs/protocol-consolidation-refactor.md` 为准。

验收：

- TypeScript、Rust、JSON Schema 和 fixtures 使用同一最终 contract。
- Collector、SDK、Transport、Processor 和 E2E 不再依赖仅用于开发阶段的双版本分支。
- API 和数据库 migration 不被无关修改。
- 完成一次明确的兼容性/升级说明。

### PR1 — CI and regression baseline

- CI 启动 PostgreSQL service 或使用独立 Compose profile。
- 执行 `pnpm db:migrate` 和 `pnpm test:integration`。
- 执行 `pnpm e2e:analytics`，保留 `pnpm e2e:dashboard`。
- 验证空数据库首次 migration、已有数据库升级和重复 migration。
- 上传 integration/E2E 日志，失败时保留 PostgreSQL、Collector、Processor 和 API 日志。
- 统一 CI 与本地 release candidate 使用的命令。

### PR2 — Browser compatibility and SDK quality

固定第一版支持矩阵：

| 浏览器 | CI 覆盖 | 范围 |
| --- | --- | --- |
| Chromium | 必须 | SDK 初始化、consent、storage、navigation、transport |
| Firefox | 必须 | 同上 |
| WebKit/Safari compatibility | 必须 | 同上 |
| 旧版浏览器 | 不承诺 | 明确不支持和失败行为 |

同时增加：

- storage 异常、`crypto.randomUUID` 不可用、页面导航和 flush 测试；
- SDK bundle size 初始预算、报告和 CI threshold；
- 产物入口、类型声明、source map 和 tree-shaking 检查；
- 不把测试浏览器矩阵扩展为未定义的长期兼容承诺。

### PR3 — Collector and runtime hardening

审计并测试：

- HTTP request body、header、origin 和 ingest key 限制；
- request、database acquire 和 shutdown timeout；
- PostgreSQL pool 上限、连接失败和恢复行为；
- SIGTERM/容器停止时的优雅关闭；
- health/readiness 与 migration 状态的边界；
- generic client error 和内部日志的脱敏；
- rate limit、数据库失败和重复事件的稳定响应；
- 不记录原始 User-Agent、Raw payload 或 ingest key。

PR3 不改变 V1/V2 或最终 protocol 的业务语义。

### PR4 — Retention and privacy lifecycle

先固定 retention policy，再实现 job。至少定义：

- Raw Event、normalized context、facts、generations 和 Page View aggregates 的保留关系；
- global/site-level 配置和最小允许保留周期；
- dry-run、候选数量、影响日期范围和审计记录；
- 外键安全的删除顺序和批处理大小；
- 删除后聚合重建或一致性校验；
- 中断、重试、锁竞争和失败恢复；
- 默认关闭，显式配置后才允许执行。

Raw Event 不能单独删除而继续声称派生数据完整。若无法在本阶段确定生产策略，只实现 dry-run 和一致性检查，自动删除推迟到后续阶段。

### PR5 — Deployment and npm release

新增部署和发布文档，覆盖：

- 环境变量、secret、Origin、ingest key 和数据库权限；
- migration job、schema compatibility check、服务部署和健康验证；
- backup、rollback、失败部署和旧 generation 恢复；
- Docker image 构建和版本标记；
- npm package metadata、public/private 边界、版本策略、变更记录和发布权限；
- npm provenance、CI token、tag/release 和撤回流程。

只有 PR0 完成后，SDK package 才能进入正式发布流程。

### PR6 — Release candidate verification

在干净环境执行完整 checklist，并记录结果：

- migration 首次执行、升级和重复执行；
- Collector、Processor、Analytics API 和 Dashboard workflow；
- Chromium、Firefox、WebKit；
- bundle size budget；
- disabled flag、数据库失败、重启和优雅关闭；
- retention dry-run 和数据一致性检查；
- npm package 构建、pack 内容和安装 smoke test。

## 7. CI 和验收命令

基础验证：

```bash
cargo fmt --check
pnpm check
pnpm test
pnpm format:check
pnpm build
git diff --check
```

数据库和 E2E 验证：

```bash
export DATABASE_URL=postgres://analytics:analytics@localhost:5432/analytics
pnpm db:migrate
pnpm db:migrate
pnpm test:integration
pnpm e2e:analytics
pnpm e2e:dashboard
```

Release candidate 还必须执行：

```bash
pnpm pack --dry-run
```

具体 package 发布命令、浏览器安装命令和 retention job 命令在对应 PR 的实现文档中固定，不在 Phase 7 总入口中维护第二套命令。

## 8. 文档和 ADR 更新

本阶段必须同步维护：

- `docs/phase-7-design.md`：本执行计划和退出条件；
- `docs/protocol-consolidation-refactor.md`：PR0 完成后的最终结果；
- `docs/roadmap.md`：每个 PR 的状态，不使用笼统的“已完成”；
- `docs/getting-started.md`：本地 migration、测试和部署顺序；
- 新增部署文档：生产环境 migration、健康检查、回滚和 backup；
- 必要时在 `docs/decisions/` 新增 retention、browser support、bundle budget 和 release ADR。

## 9. 风险和回滚

- Protocol consolidation 是唯一预期的 breaking refactor，必须单独提交并提供升级说明。
- migration 只能 additive；已执行 migration 不修改。
- retention 删除属于破坏性操作，必须先 dry-run，并且默认关闭。
- runtime hardening 失败时应回滚服务镜像，不回滚已成功执行的 schema migration。
- generation 和派生事实仍通过现有 active/retired 机制回滚，Raw Event 不删除。
- npm 发布失败不应影响已经部署的 Collector、Processor、API 或 Dashboard。

## 10. Phase 7 退出条件

- PR0–PR6 的验收结果和已知限制已记录。
- CI 覆盖 migration、integration、Analytics E2E、Dashboard E2E 和浏览器矩阵。
- SDK bundle size、package 内容和类型入口通过检查。
- Collector runtime failure、shutdown、redaction 和 readiness 行为有测试。
- retention policy 已批准；若未批准，自动删除明确延期且 dry-run 仍安全。
- 部署、backup、migration、rollback 和 npm release 文档可由新环境执行。
- release candidate checklist 在干净环境完整通过。

Phase 7 完成后，现有 Phase 6 Analytics workflow 达到稳定 release candidate。Phase 8 再补齐当前定义的 MVP 产品能力；新的统计能力和产品范围扩展必须进入后续 Phase。
