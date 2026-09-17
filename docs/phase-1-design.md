# Phase 1 Design — Client SDK

> Status: Working design
> Scope: Framework-agnostic Client SDK, NavigationObserver, Next.js App Router Adapter, Browser Context and basic Transport

## 1. Phase 1 定义

Phase 1 在 Phase 0 的 Monorepo、Router Playground 和 Event Protocol V1 之上，建立第一个可以在浏览器中运行的 Client SDK。

目标是打通：

```text
Next.js App Router
  → observer-next
  → analytics-core
  → PageViewEvent
  → Browser Context
  → Buffer
  → Transport
```

Phase 1 不依赖 Backend、数据库或 Dashboard。发送能力使用可替换的 Transport 和 Mock Transport 验证，真实 Collector 在 Phase 2 实现。

## 2. 目标

### 2.1 必须达成

- 创建具有实际实现内容的 Client SDK packages。
- 保持 Analytics Core 与具体 Framework、Router 和浏览器 API 解耦。
- 定义并实现通用 `NavigationObserver`。
- 只实现 Next.js App Router Adapter。
- 将导航转换为符合 Event Protocol V1 的 PageViewEvent。
- 支持手动 `pageview()` 和自动导航观察。
- 支持基础 Browser Context 采集。
- 提供可替换的 Transport 接口。
- 实现内存 Buffer、批量 flush 和 `beforeSend()` 扩展点。
- 将 SDK 接入 Router Playground，形成可观察的纵向 workflow。

### 2.2 不属于本阶段

- 不创建或实现 Backend Collector。
- 不连接 PostgreSQL、Processor、Analytics API 或 Dashboard。
- 不创建 Rust workspace 或 Rust package。
- 不实现 React Router、TanStack Router 或其他 Router Adapter。
- 不实现服务端事件或 Server SDK。
- 不实现 Visitor ID、Session 持久化、跨设备识别或精确归因。
- 不实现离线持久化、复杂重试、Exactly-once 或可靠送达保证。
- 不实现 Conversion、Funnels、Replay、Heatmap、A/B Testing 或 Web Vitals。

## 3. 实施原则

### 3.1 线性创建实际模块

Phase 1 不一次性创建所有最终目录。每个 PR 只创建当前有实现和测试的 package：

```text
Protocol TypeScript types
  → Observer contract
  → Analytics Core
  → Next.js Adapter
  → Browser Context
  → Transport / Buffer
```

### 3.2 JSON Schema 是协议事实来源

`protocol/schemas/` 继续作为跨语言协议的事实来源。`protocol-ts` 只提供 TypeScript 消费类型，不替代 JSON Schema，也不改变协议字段语义。

### 3.3 Observer 不负责 Analytics

Router Adapter 只观察导航并输出 `NavigationEvent`，不负责：

- Page View 判定之外的统计逻辑
- Session 或 Visitor
- Context 采集
- Buffer
- Transport

### 3.4 浏览器能力按需读取

SDK package 可以被 SSR 应用导入。`window`、`document`、`navigator` 等对象只能在浏览器运行时读取，模块加载和服务端渲染不能崩溃。

## 4. Phase 1 目标目录

完成 Phase 1 后，实际创建：

```text
packages/
├── protocol-ts/
├── observer-core/
├── observer-next/
├── analytics-core/
├── analytics-browser/
└── transport/
```

Package 都加入 pnpm workspace，并包含自己的 `package.json`、`src/` 和测试。

不在 Phase 1 创建：

```text
services/
crates/
apps/dashboard/
collector
storage
processor
analytics-api
```

## 5. 公共接口

### 5.1 NavigationObserver

定义在 `observer-core`：

```ts
export type NavigationType = "initial" | "push" | "replace" | "pop" | "unknown";

export interface NavigationEvent {
  url: string;
  path: string;
  title?: string;
  referrer?: string;
  navigationType: NavigationType;
  occurredAt: number;
}

export interface NavigationObserver {
  subscribe(listener: (event: NavigationEvent) => void): () => void;
}
```

约定：

- 首次加载产生 `initial`。
- pathname 或 search params 变化产生导航事件。
- push、replace、back、forward 分别尽可能映射到对应类型。
- 无法判断时使用 `unknown`，不能阻塞事件流。
- hash 变化只由 Adapter 观察，不默认生成 Page View。
- `subscribe()` 支持多个 listener。
- 返回的 unsubscribe 可以安全重复调用。

### 5.2 Transport

定义在 `transport` 或 SDK 公共类型中：

```ts
export interface Transport {
  sendBatch(events: readonly PageViewEvent[]): Promise<void>;
}
```

实现：

- `MockTransport`：测试和 Playground 使用。
- `FetchTransport`：向配置的 endpoint 发送 JSON batch。
- `BeaconTransport`：页面卸载等场景使用 `navigator.sendBeacon`。

Transport 不负责 Schema 业务决策、Site 校验、鉴权或重试策略。Collector 的安全和请求校验在 Phase 2 实现。

### 5.3 Analytics API

SDK 对外提供最小生命周期 API：

```ts
export interface Analytics {
  observe(observer: NavigationObserver): () => void;
  pageview(): void;
  flush(): Promise<void>;
  destroy(): void;
}
```

初始化选项至少包含：

```ts
export interface AnalyticsOptions {
  siteId: string;
  transport?: Transport;
  beforeSend?: (event: PageViewEvent) => PageViewEvent | null;
}
```

约定：

- `beforeSend()` 返回 `null` 时丢弃事件。
- `observe()` 返回的 unsubscribe 只解除本次订阅。
- `destroy()` 解除订阅并阻止后续事件进入 Buffer。
- `flush()` 发送当前 Buffer；空 Buffer 是成功的 no-op。
- 同一 SDK 实例只维护自己的 Buffer 和生命周期。

## 6. Page View 语义

首期 Page View 规则：

```text
initial load       → PageViewEvent
pathname change    → PageViewEvent
search params change → PageViewEvent
push / replace     → PageViewEvent
back / forward     → PageViewEvent
hash-only change   → 不生成 PageViewEvent
```

事件构造规则：

- `type` 固定为 `page_view`。
- `schema_version` 固定为 `1`。
- `event_id` 由客户端生成 ULID。
- `occurred_at` 使用当前 Unix milliseconds。
- `site_id` 使用 SDK 初始化配置。
- `url`、`path`、`title` 和 `referrer` 来自当前浏览器上下文。
- 浏览器附加信息放入 `context`。
- 连续完全相同的导航事件做简单去重。
- 不在 Client SDK 中计算 Session 或 Visitor。

## 7. Browser Context

`analytics-browser` 负责读取浏览器可直接取得的字段：

- URL / pathname
- document title
- referrer
- UTM 参数
- language
- timezone
- viewport / screen size
- 基础 User-Agent 信息

要求：

- Context provider 是可替换的接口。
- 单项 API 缺失时使用 undefined 或安全缺省值。
- 不采集 IP、精确 Geo、指纹或跨设备标识。
- 不将 Context provider 与 Next.js 类型绑定。
- Browser Context 与 NavigationEvent 的基础 URL 信息合并时，当前导航事件值优先。

## 8. Buffer 和 flush

首期使用内存 Buffer：

- 默认单批最多 20 条事件。
- 达到 20 条时自动 flush。
- `flush()` 可主动发送不足一批的事件。
- flush 过程中继续产生的事件进入下一批。
- 空 Buffer flush 成功返回。
- Transport 失败时保留未成功发送的事件，并返回错误。
- 不使用 localStorage、IndexedDB 或 Service Worker 持久化。
- 不实现复杂重试和指数退避。

单批上限低于 Protocol V1 的 100 条限制，避免浏览器端请求过大；Backend 仍以 Protocol Schema 和自身请求限制为准。

## 9. PR 划分

Phase 1 按以下顺序线性实施。

### PR1 — Client SDK contracts and core

分支：`phase-1/pr1-client-sdk-core`

创建 `protocol-ts`、`observer-core` 和 `analytics-core`，实现类型、Observer 契约、Page View 构造、`beforeSend()` 和 fake observer 测试。不实现 Next.js、浏览器读取和网络发送。

### PR2 — Next.js App Router Adapter

分支：`phase-1/pr2-nextjs-observer`

创建 `observer-next`，实现 App Router initial、pathname、query、push、replace、back / forward 观察，并将 Observer 输出接入 Playground 的调试区域。不实现 Transport、Buffer 或统计逻辑。

PR2 的接入组件为 `NextNavigationBridge`：它在 Client Component 中接收 `onNavigation` 回调，输出标准 `NavigationEvent`。Adapter 使用 App Router hooks 获取路由状态，使用 History API 和 `popstate` 辅助识别导航来源。hash-only 变化只保留在 Playground 的原始 Router 观察中，不输出标准导航事件。

### PR3 — Browser SDK runtime and context

分支：`phase-1/pr3-browser-sdk-runtime`

创建 `analytics-browser`，实现 `createAnalytics()`、手动 `pageview()`、Observer 订阅、Browser Context 和 SSR 安全。不连接真实 Backend，使用 fake observer 和 mock transport 测试。

### PR4 — Transport, Buffer and workflow

分支：`phase-1/pr4-transport-and-flush`

创建 `transport`，实现 Mock、Fetch、Beacon、Buffer、自动批量 flush 和主动 flush。完成 Playground 中的 SDK workflow 验证，并确认生成事件符合 Event Protocol V1。

## 10. 测试策略

Phase 1 引入 Vitest，优先使用单元测试和可控 fake 实现，不提前引入完整浏览器 E2E。

必须覆盖：

- Observer 多 listener、unsubscribe 和重复 unsubscribe。
- initial、pathname、query、push、replace、pop 和 unknown 导航类型。
- hash-only 导航不会生成 Page View。
- PageViewEvent 符合 Protocol V1。
- `beforeSend()` 修改和丢弃事件。
- SSR 环境导入 SDK 不访问浏览器对象。
- 浏览器 Context 缺少单项 API 时仍可生成事件。
- `pageview()` 和 Observer 自动事件共用同一管线。
- Buffer 达到 20 条自动 flush。
- 手动 flush、空 flush 和 flush 失败保留事件。
- Mock、Fetch、Beacon Transport 可以替换。
- destroy 后不会继续处理导航事件。

Playground 继续使用 Phase 0 的人工导航矩阵；必要时在 Phase 1 后续再增加 Playwright。

## 11. 文档和脚本

每个 PR 必须同步：

- package README 或 API 文档
- 相关测试和 fixtures
- 根 workspace 配置
- 根目录统一 format、check、test、build 脚本
- Protocol 变更对应的 Schema、TypeScript 类型和 examples / fixtures

Phase 1 仍复用 Phase 0 的 Docker Compose Playground，不加入 Backend、数据库或新的业务服务。

## 12. Phase 1 验收清单

### SDK 与接口

- [x] `protocol-ts` 与 Protocol V1 一致。
- [x] `observer-core` 不依赖具体 Router。
- [x] `analytics-core` 不依赖 React、Next.js 或浏览器 API。
- [ ] `createAnalytics()`、`pageview()`、`observe()`、`flush()`、`destroy()` 可用。
- [x] `beforeSend()` 可以修改或丢弃事件。

### Next.js Adapter

- [x] Next.js App Router initial 可以观察。
- [x] pathname、query、push、replace、back / forward 可以观察。
- [x] hash-only 变化不会生成标准 NavigationEvent。
- [x] Playground 可以显示 Adapter 观察结果。

### Browser 与 Transport

- [ ] Browser Context 可以安全采集。
- [ ] SSR 导入不会崩溃。
- [ ] Mock、Fetch、Beacon Transport 可以替换。
- [ ] Buffer 和 flush 行为符合约定。
- [ ] 生成事件通过 Event Protocol V1 校验。

### 工程

- [x] 每个当前 package 都有测试和独立构建入口。
- [x] Host 检查和 CI 都能运行 Phase 1 测试。
- [x] 不创建 Backend、Storage、Dashboard 或 Rust workspace。
- [x] Playground 的现有功能保持可用。

## 13. Phase 1 退出条件

满足以下条件后进入 Phase 2：

```text
Next.js App Router
  → observer-next
  → Client SDK
  → PageViewEvent
  → Buffer
  → Mock / Fetch / Beacon Transport
```

并且：

- 生成事件全部符合 Event Protocol V1。
- SDK 可以在没有 Backend 的情况下通过 Mock Transport 完整测试。
- Transport endpoint 可配置，后续 Collector 可以直接接入。
- Analytics Core 不依赖 Next.js、React 或浏览器 API。
- 其他 Router 只需实现 NavigationObserver 即可接入。
- 测试、文档和 CI 均完成。

## 14. 后续允许调整的内容

以下内容可以根据真实 SDK 使用结果调整：

- `beforeSend()` 是否支持异步函数。
- 默认 Buffer 大小和 flush 时机。
- Beacon 的默认启用策略。
- Browser Context 的具体字段结构。
- Next.js Adapter 的实现载体，例如组件或 Hook。
- 是否在后续阶段加入 Playwright 和真实 Backend 集成测试。

以下边界不应在 Phase 1 随意改变：

- Analytics Core 不依赖具体 Router。
- Router Adapter 不负责统计和 Transport。
- Client 与 Backend 使用 Event Protocol V1 通信。
- Phase 1 不创建 Backend、Storage、Dashboard 或 Rust workspace。
