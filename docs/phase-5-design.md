# Phase 5 Design — Analytics Semantics and Identity

> Status: PR1 contract review complete; Phase 5 implementation work not started
> Scope: Visitor、Session、时间语义、Browser Context 和 Analytics Dimensions 的契约设计

## 1. Phase 5 定义

Phase 5 不直接扩展当前 Page View Processor，而是先固定下一阶段 Analytics 能力需要依赖的语义和跨语言契约：

    Event Protocol / Browser SDK
      → Visitor identity contract
      → Session semantics
      → Browser Context schema
      → Dimension and API contract
      → canonical fixtures and migration plan

Phase 5 的产物必须能够回答：同一个人、同一个 Session、同一个事件和同一个浏览器维度在系统中分别代表什么，以及当事件迟到、重复、跨午夜或缺少匿名 ID 时如何处理。

Visitor、Session 和维度聚合的生产实现留到 Phase 6。没有通过本阶段契约评审前，不修改 Processor 统计语义，不新增正式聚合表，也不在 Dashboard 中添加对应指标。

## 2. 必须达成

- 固定匿名 Visitor 的生成、作用域、生命周期和存储边界。
- 固定 Session inactivity timeout、跨午夜、跨标签页和事件乱序语义。
- 明确 occurred_at、received_at、迟到事件、重复事件和重处理规则。
- 固定 Browser Context 的字段、类型、版本和隐私边界。
- 固定 Referrer、UTM、Language、Timezone、Device、Browser、OS 的定义。
- 设计 Protocol、SDK、Storage、Processor、Analytics API 和 Dashboard 的迁移边界。
- 建立可以独立验证上述规则的 canonical fixtures。
- 添加对应 ADR，并把 Phase 6 的实现入口和退出条件写清楚。

## 3. 不属于本阶段

- 正式实现 Visitor ID 生成或持久化。
- 正式实现 Sessionization、Visitor/Session 聚合或 Dashboard 指标。
- Country、IP 持久化、精确地理位置、指纹识别或跨设备识别。
- 登录用户 ID、组织身份、广告 ID 或第三方身份合并。
- Realtime Session、分布式 Session claim/lease 或复杂迟到事件修正。
- 新增图表库、客户端缓存库或新的 Dashboard 数据聚合逻辑。
- 在没有契约的情况下修改 Event Protocol V1 的必填字段。

## 4. 设计原则

### 4.1 Page View 语义保持兼容

Visitor 和 Session 是 Page View 之上的新维度，不能改变现有 Page View 的计数、幂等键、UTC 日聚合和 Analytics API contract。

缺少新身份字段的旧事件仍然可以被 Collector 接收并计入 Page Views，但不能被计入 Visitor 或 Session 指标。这样可以让 Protocol V1 数据平滑迁移到新契约。

### 4.2 Phase 3/4 兼容性边界

Phase 5 PR1 不改变已经交付的 Page View 行为：

- Protocol V1 事件继续使用现有的 `site_id + event_id` 幂等规则。
- `occurred_at` 仍是 Page View 行为时间，日报日期仍按 UTC calendar date 计算。
- `received_at` 继续由 Collector 生成，用于接收诊断、迟到窗口判定和 freshness，不替代 `occurred_at`。
- 现有 Page View totals、daily、routes、Overview、Timeline 和 Top Pages contract 保持不变。
- V1 事件即使没有 Visitor ID，也必须继续计入 Page Views，但不得被合并到任意共享的 anonymous fallback Visitor。
- Phase 6 新增的 Visitor、Session 和 Dimension 聚合只能作为派生能力接入，不能要求 Phase 3/4 立即修改生产表或 API。

Phase 5 PR1 只冻结实现边界；Protocol V2、canonical fixtures、migration 和新的 API endpoint 分别留给后续 PR。

### 4.3 身份是匿名且站点隔离的

Visitor ID 是站点范围内的随机匿名标识，不代表真实身份，也不能通过 IP、User-Agent、屏幕尺寸或其他 Browser Context 字段推导。

同一个浏览器只在同一个 site_id 范围内复用 Visitor ID。不同站点不能共享 Visitor ID，也不通过跨站 Cookie、URL 参数或服务端 IP 做关联。

### 4.4 Raw Event 仍是事实源

所有 Visitor、Session 和维度结果都必须能从 Raw Event 重建。聚合表、派生字段和解析器版本不能替代原始事件。

    raw_events
      ├── page_view_daily / page_view_routes
      ├── visitor_daily / visitor_totals
      ├── session_daily / session_totals
      └── dimension aggregates

Phase 6 可以选择增量处理或受影响 Visitor 重建，但不能只依赖不可重建的计数器。

### 4.5 时间语义显式区分

- occurred_at：浏览器产生事件的 Unix milliseconds，是行为时间。
- received_at：Collector 接收并通过校验后生成的服务端时间，是接收时间。
- 报表日期：默认取 occurred_at 的 UTC calendar date。
- received_at 不用于替代 occurred_at 计算 Page View、Visitor 或 Session。
- Event Protocol V2 允许客户端时钟最多领先 received_at 5 分钟；超过该范围的事件拒绝为 invalid_occurred_at。

## 5. Visitor Identity Contract

### 5.1 ID 生成

Phase 6 的 Browser SDK 为每个站点生成一个不可预测的随机匿名 ID。采用随机 UUID v4 的小写 canonical ASCII 表示，长度固定为 36 个字符。ID 不包含时间、域名、Email、User ID 或可逆编码信息。

服务端只校验 ID 的格式、长度和字符集，不尝试从 ID 解码身份信息。格式一旦确定，应作为 Protocol contract 的版本化约束，而不是依赖某一个语言的 UUID 实现细节。

### 5.2 客户端存储

Visitor ID 使用被观测网站的第一方 localStorage，以支持同一 origin 内的页面刷新、跨标签页和同源路径复用。Phase 6 中一个 site_id 只对应一个浏览器 origin；多个 origin 必须使用不同的 site_id。Dashboard 不生成、不读取也不写入被观测网站的 Visitor ID。默认不使用 Cookie、URL、跨站存储或指纹作为 fallback。

选择 localStorage 的原因是第一阶段只需要浏览器 SDK 在客户端复用匿名 ID，不需要服务端从 Cookie 读取身份，也不会让 Visitor ID 自动附加到每个 HTTP 请求。未来如果出现跨子域或服务端协作需求，再通过独立 ADR 评估 Cookie 方案。

以下情况会产生新的 Visitor：

- 用户清除站点存储。
- 浏览器启用阻止或隔离站点存储。
- 使用隐私模式且会话结束后存储被清除。
- 站点或 SDK 显式轮换匿名 ID。

Consent 和存储失败规则如下：

- 没有 analytics consent 或用户已经 opt out：SDK 不生成 Visitor ID，也不发送 Analytics Event。
- 已有 consent 但 localStorage 不可用：SDK 可以发送没有 Visitor ID 的 Page View，事件不计入 Visitor/Session 指标。
- 用户撤回 consent：SDK 停止发送，并在可访问时删除本 origin 的 Visitor ID。
- 不使用临时 ID、User-Agent、IP 或指纹作为 fallback。

跨子域 Visitor 共享和服务端协作不属于 Phase 6；未来若需要，必须通过新的 ADR 评估 Cookie 或其他方案。

### 5.3 Event 表达

Visitor ID 应作为版本化 Event Protocol 字段表达，而不是埋在任意 context key 中。确定的兼容扩展为可选字段：

    {
      "schema_version": 2,
      "event_id": "01...",
      "type": "page_view",
      "site_id": "site_playground",
      "visitor_id": "anonymous-site-scoped-id",
      "occurred_at": 1780000000000,
      "path": "/about",
      "context": {}
    }

Protocol V2 增加 visitor_id 作为可选的正式字段，Protocol V1 继续被 Collector 接受，并保持现有 Page View 语义。V1 事件没有 Visitor ID 时仍可计入 Page Views，但不计入 Visitor 或 Session 指标。session_id 不由浏览器作为可信统计输入发送；Session 由 Processor 根据 Visitor ID 和事件时间确定。

## 6. Session Semantics

### 6.1 默认规则

Session 是同一 site_id + visitor_id 下的一组 Page View 事件，按 occurred_at 排序后满足以下规则：

- 默认 inactivity timeout：30 分钟。
- 相邻事件间隔小于 30 分钟，属于同一 Session。
- 相邻事件间隔大于或等于 30 分钟，开始新的 Session。
- Session 的首个事件时间为 started_at，最后一个事件时间为 ended_at。
- Session ID 是服务端派生的稳定标识，不信任客户端提供的 Session ID。

相同时间戳的事件按 event_id 做确定性次序处理，避免重处理结果依赖数据库返回顺序。

### 6.2 跨午夜

Phase 3/4 的默认报表时区是 UTC，因此 Session 在 UTC 日期边界拆分：即使相邻事件间隔不超过 30 分钟，跨越 UTC 00:00 也开始新的 Session。

这使得每日 Session 报表与现有 UTC Page View Timeline 一致。未来支持站点时区时，必须新增明确的报表时区 contract，不能隐式改变当前 UTC 语义。

### 6.3 跨标签页和导航

同一站点、同一浏览器存储范围内的标签页共享 Visitor ID，因此多个标签页产生的事件可以属于同一 Session。Session 不按标签页、pathname、referrer 或导航类型拆分。

initial、push、replace、pop 都是 Page View 事件的来源，不改变 Session 规则。Hash 是否产生 Page View 仍遵循现有 Observer/SDK 契约，不由 Session 层重新判断。

### 6.4 重复、乱序和迟到事件

- 重复事件沿用 site_id + event_id 幂等规则，只计算一次。
- 事件按 occurred_at 而不是 received_at 参与 Sessionization。
- 迟到事件可能插入已有 Session、拆分 Session 或合并原本相邻的事件段。
- Processor 必须能够重建受影响 Visitor 的 Session 结果；不能只对累计 Session total 做不可逆加法。
- Phase 6 初期采用 24 小时 lateness window。窗口按事件的 arrival delay 定义：received_at - occurred_at 小于或等于 24 小时的事件可以触发自动 Session 重建。Collector 仍然接收合法的迟到事件；超过窗口的事件保留在 Raw Event 中，但通过显式 backfill/rebuild 修正，不自动修改已经稳定的历史 Session 聚合。occurred_at 晚于 received_at 但不超过 5 分钟的未来事件不计为迟到，超过 5 分钟则拒绝为 invalid_occurred_at。

### 6.5 缺少 Visitor ID

没有 Visitor ID 的事件仍然计入 Page Views，但不计入 unique visitors、sessions 或依赖身份的维度。它们不能共享一个固定的 anonymous fallback ID，否则会把所有未知浏览器错误合并成一个 Visitor。

## 7. Browser Context Contract

### 7.1 字段分类

Phase 6 允许采集以下浏览器可直接取得的字段：

| 类别 | 字段 | 语义 |
| --- | --- | --- |
| Locale | language | 浏览器首选语言，使用 BCP 47 字符串，不推导国家 |
| Timezone | timezone | IANA timezone 名称；缺失或无效时为 unknown |
| Viewport | viewport_width, viewport_height | 事件产生时 CSS viewport 像素，非身份标识 |
| Screen | screen_width, screen_height | 浏览器报告的 screen 像素，非身份标识 |
| Campaign | utm_source, utm_medium, utm_campaign, utm_term, utm_content | URL query 中的 allowlist 参数 |
| Referrer | referrer | 最多 4096 字符，按长度和安全规则处理 |
| User Agent | user_agent | 可选原始浏览器声明，只用于服务端解析，不直接作为 Dashboard 维度输出 |

### 7.2 稳定 schema 和版本

Browser Context 的字段名、类型、最大长度和缺失语义必须进入 Protocol schema。未知字段可以按照现有 forward-compatible 规则保留在 Raw Event，但不能未经设计直接成为聚合维度。

Protocol V2 使用独立的 context_schema_version，与 Event Protocol schema_version 分开：协议版本表示事件 envelope，context 版本表示浏览器字段的解释方式。V2 中存在 context 时使用 context_schema_version = 1；没有 context 时可以省略该字段。

所有派生 Device、Browser、OS 值都必须带 parser/version 语义，避免升级 User-Agent parser 后无法解释历史结果。未知值统一归入 unknown，不通过猜测填充。

Browser Context 的首个版本固定以下约束：

| 字段 | 类型和约束 | 缺失/非法值 |
| --- | --- | --- |
| language | BCP 47 字符串，最多 64 字符 | unknown |
| timezone | IANA 名称，最多 64 字符 | unknown |
| viewport_width, viewport_height | 整数，范围 0–100000 | unknown |
| screen_width, screen_height | 整数，范围 0–100000 | unknown |
| utm_source, utm_medium, utm_campaign, utm_term, utm_content | 字符串，每项最多 256 字符 | 不产生该维度 |
| user_agent | 字符串，最多 1024 字符 | unknown |

非法字段不会导致整个事件失败；字段被丢弃并按上表处理。超过最大长度的文本按字段拒绝，不截断后继续作为统计值使用。

### 7.3 Referrer 语义

Raw Event 可以保留经过长度和安全规则处理的原始 referrer，但 API 和 Dashboard 默认按 referrer host 聚合：

    https://news.example.com/article?id=123#comments
      → news.example.com

Host 规范化规则为：解析 URL 的 hostname、转换为小写、移除末尾点和默认端口；不保留 scheme、userinfo、path、query 或 fragment。解析失败、空值和浏览器没有 Referrer 的事件统一归类为 direct。完整 URL、query 参数和 fragment 不作为默认维度输出，以避免高基数和敏感信息泄露。

### 7.4 隐私和数据边界

- 不采集 IP 到 Raw Event 或聚合表。
- 不采集精确地理位置、Canvas、字体列表、硬件并发数或其他指纹字段。
- 不把 User-Agent 原文返回给 Dashboard；只返回预定义、低基数的派生分类。
- UTM、Referrer 和文本字段必须有最大长度，并在 API/日志中避免泄露完整敏感 query。
- Language 和 timezone 是浏览器设置，不等同于用户国家或实际位置。
- Visitor ID 只作为站点内匿名计数键，不允许通过 API 查询单个 Visitor 的完整浏览轨迹。

## 8. Dimension Definitions

Phase 6 允许的第一批维度定义如下：

- Language：navigator.language 的规范化 BCP 47 值；缺失为 unknown。
- Timezone：浏览器提供的 IANA 名称；不从 IP 推断。
- UTM：只读取五个 allowlist key，按字符串维度统计；空值不产生维度项。
- Referrer：保留受限的原始值用于事实记录，报表默认按规范化 host 统计；缺失值为 direct。
- Device：从 User-Agent 解析出的 desktop、mobile、tablet 或 unknown。
- Browser：解析器定义的稳定 browser family 和 major version；无法识别为 unknown。
- OS：解析器定义的稳定 OS family 和 major version；无法识别为 unknown。

这些分类都不是 Visitor identity，也不能用于去重 Visitor。Visitor 和 Session 统计必须使用明确的身份键。

## 9. Processing and Storage Design Boundary

Phase 5 只固定设计，不创建正式 migration。Phase 6 实现时至少需要评估：

    raw_events
      + visitor_id / context_schema_version / parsed_context_version

    visitor_daily or visitor aggregates
    session records or session aggregates
    dimension aggregates

具体采用事件级 Session 表、Visitor 级重建表还是按日聚合表，必须满足：

- 可从 Raw Event 重建。
- 重复处理幂等。
- 迟到事件的受影响范围可识别。
- parser 版本变更后可以重新解析。
- 不改变现有 Page View 聚合和 API contract。

Processor 仍然负责统计转换，Collector 只做验证、接收和持久化。Collector 不生成 Session，不访问 Browser Context 以外的身份信息，也不执行复杂聚合。

## 10. Analytics API and Dashboard Contract

Phase 5 冻结 API contract，Phase 6 实现 endpoint。新增 API 必须延续现有路径结构、站点隔离和 UTC 日期范围：

    GET /v1/sites/{site_id}/reports/{from}/{to}/visitors
    GET /v1/sites/{site_id}/reports/{from}/{to}/sessions
    GET /v1/sites/{site_id}/reports/{from}/{to}/dimensions/{dimension}?limit=20

Visitors 和 Sessions 的响应固定为：

    {
      "site_id": "site_playground",
      "from": "2026-09-01",
      "to": "2026-09-18",
      "page_views": 42,
      "unique_visitors": 12,
      "sessions": 15,
      "items": [
        { "day": "2026-09-01", "page_views": 3, "unique_visitors": 2, "sessions": 2 }
      ],
      "data_as_of": "2026-09-18T12:00:00Z",
      "aggregation_version": 1
    }

Dimension API 的响应固定为：

    {
      "site_id": "site_playground",
      "from": "2026-09-01",
      "to": "2026-09-18",
      "dimension": "referrer_host",
      "items": [
        { "value": "news.example.com", "page_views": 20, "unique_visitors": 8, "sessions": 9 }
      ],
      "data_as_of": "2026-09-18T12:00:00Z",
      "aggregation_version": 1
    }

API 规则：

- unique visitors
- sessions
- items 按日期升序；Dimension items 按 page_views desc, value asc 排序。
- Dimension 默认 limit 为 20，最大 limit 为 100；Phase 6 不实现分页。
- data_as_of 表示构成本响应全部数值的最新 processed received_at，使用 UTC RFC 3339。
- aggregation_version 表示 Visitor/Session/Dimension 计算语义版本。
- 空结果返回 200 和空 items；未知分类使用 unknown，无 Referrer 使用 direct。

API 不返回 Visitor ID、原始 User-Agent、原始 IP 或单个 Visitor 的逐事件轨迹。Dashboard 只消费 API，并且必须明确展示数据范围和可能的延迟语义。

## 11. Canonical Fixtures

Phase 5 必须新增与现有 Phase 3 fixtures 分离的语义 fixtures。至少包含：

1. single-visitor-session：同一 Visitor 的多个页面浏览属于一个 Session。
2. session-timeout：间隔超过 30 分钟后产生第二个 Session。
3. midnight-split：跨 UTC 午夜即使间隔小于 timeout 也拆分 Session。
4. multi-tab-shared-visitor：多个标签页共享 Visitor，但不按标签页拆分 Session。
5. duplicate-and-late-events：重复事件只计算一次，迟到事件按 occurred_at 重建结果。
6. missing-visitor-id：Page Views 保留，Visitor/Session 指标不增加。
7. browser-context：覆盖语言、timezone、viewport、UTM、referrer 和 unknown 值。
8. privacy-boundary：确认 IP、精确地理位置和未允许字段不进入派生 API 响应。

每个 fixture 必须同时声明：输入事件、事件接收顺序、预期 Visitor/Session 结果、预期 Page View 结果、预期维度结果和适用的 schema/parser 版本。

## 12. PR 拆分

### PR1 — Semantics and ADR

完成：

- 本文档。
- Visitor、Session、时间和隐私边界 ADR。
- 与 Phase 3/4 现有语义的兼容性说明。
- Phase 6 实现前必须冻结的开放问题清单。

不修改生产代码、数据库 schema 或 API。

### PR2 — Protocol and SDK Contract Fixtures

完成：

- Visitor ID 和 Browser Context 的版本化 schema 草案。
- V1 compatibility / V2 migration fixture。
- Browser Context 字段类型、长度和 unknown 规则的 validation fixture。
- 事件接收顺序、迟到事件和重复事件的 canonical input fixture。

本 PR 只添加契约、示例和验证，不生成生产 Visitor ID 或 Session。

### PR3 — API and Phase 6 Implementation Plan

完成：

- Visitor、Session、Dimension API response contract。
- Storage/Processor migration plan。
- 数据新鲜度、重建、parser version 和回滚策略。
- Phase 6 的实现拆分、验收命令和 E2E workflow 设计。

## 13. 测试策略

### Contract tests

- Visitor ID 格式、长度、站点作用域和缺失语义。
- Browser Context schema、字段类型、长度和 unknown 值。
- V1 事件仍然可以产生原有 Page View 结果。
- V2 扩展字段不会改变既有 Collector 的幂等行为。

### Semantic fixture tests

- Session timeout 的边界值：小于 30 分钟、恰好 30 分钟和超过 30 分钟；恰好 30 分钟开始新的 Session。
- UTC 午夜前后事件。
- 同一 Visitor 的跨标签页事件。
- 乱序、重复、迟到事件和重处理。
- 缺少 Visitor ID 的降级行为。

### Privacy tests

- 禁止字段不会进入 Protocol、Raw Event 派生维度或 API 响应。
- User-Agent 原文不出现在 Dashboard/API response。
- 站点之间不能通过 Visitor ID 或查询参数串联。

### Exit validation

Phase 5 不要求新的生产 E2E 服务，但必须能运行：

    pnpm protocol:validate
    pnpm test
    pnpm check
    git diff --check

Phase 6 开始前，还必须完成新 fixtures 的跨 TypeScript/Rust 读取验证，并由实现 PR 补充数据库和浏览器 E2E。

## 14. Phase 5 退出条件

Phase 5 只有在以下条件全部满足时结束：

- Visitor ID、Session、时间和 Browser Context 语义已评审并记录。
- V1 兼容策略和需要的 Protocol 版本边界已确定。
- 存储、Processor、API 和 Dashboard 的责任边界已确定。
- canonical fixtures 可以覆盖正常、边界、迟到、重复和隐私场景。
- Phase 6 不需要重新解释 30 分钟 timeout、UTC 午夜或 Visitor 缺失语义。
- 所有开放问题要么已决策，要么明确标记为不属于 Phase 6。

Phase 5 结束后进入 Phase 6：Browser and Analytics Dimensions。

## 15. Phase 6 前置决策

以下内容不属于 Phase 5 PR1 的稳定生产 contract，必须在 Phase 6 实现前单独决定：

- User-Agent parser 的实现、版本锁定、升级和历史重解析策略。

在该决策冻结前，Phase 6 只能实现测试 fixture 或实验代码，不能把某个 parser 或版本当作稳定的生产 contract。该开放项不阻塞本 PR，因为 PR1 已经冻结了 User-Agent 原文不进入 Dashboard/API、派生值必须带 parser/version 语义以及未知值归入 `unknown` 的边界。
