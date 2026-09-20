# ADR-005：Visitor 和 Session 采用站点隔离的匿名身份与确定性服务端 Sessionization

- Status: Accepted (Phase 5)
- Date: 2026-09-20

## Context

Phase 3/4 只统计 Page Views，没有稳定的匿名 Visitor 标识、Session 规则或 Browser Dimensions。直接在 Processor 中增加去重和 Session 计数会把未评审的产品语义固化到数据库和聚合代码中。

系统还需要兼容已经存在的 Protocol V1 事件，并能够处理重复、乱序、迟到和跨 UTC 午夜事件。

## Decision

Phase 5 先固定以下设计方向，Phase 6 再实现：

1. Visitor ID 使用浏览器生成的随机 UUID v4、站点隔离、匿名标识。它不从 IP、User-Agent、屏幕尺寸或其他 Browser Context 推导。
2. Visitor ID 存储在被观测网站的第一方 localStorage 中，一个 site_id 在 Phase 6 只对应一个 origin。Dashboard 不生成、不读取也不写入该 Visitor ID。第一阶段不使用 Cookie、URL 参数、跨站存储或指纹作为 fallback。
3. Visitor ID 作为版本化事件字段表达，而不是依赖任意 context key；缺失 Visitor ID 的旧事件仍计入 Page Views，但不计入 Visitor/Session 指标。
4. Session 由服务端根据 site_id + visitor_id + occurred_at 确定性计算，不信任客户端 Session ID。
5. 默认 inactivity timeout 为 30 分钟；间隔大于或等于 30 分钟开始新 Session；跨 UTC 午夜强制拆分。
6. 重复事件沿用 site_id + event_id 幂等规则；迟到事件按 occurred_at 参与重建，而不是按 received_at 直接追加。Phase 6 初期采用 24 小时 arrival-delay lateness window，即 received_at - occurred_at 小于或等于 24 小时；窗口外事件通过显式 backfill/rebuild 修正。
7. Referrer 的 Raw Event 值经过长度和安全处理，Analytics API 和 Dashboard 默认按小写、去末尾点和默认端口的规范化 host 聚合；缺失或解析失败的值归类为 direct。
8. Protocol V2 增加可选 visitor_id 正式字段，Collector 同时兼容 V1 和 V2；V1 事件继续计入 Page Views，但没有 Visitor ID 时不计入 Visitor/Session 指标。
9. Browser Context 采用 context_schema_version = 1、固定类型/长度和显式 unknown 值。没有 analytics consent 时不发送事件；User-Agent 原文不作为 Dashboard/API 输出，不采集 IP、精确地理位置或指纹字段。
10. Protocol V2 允许客户端时钟最多领先 received_at 5 分钟；超过该范围的事件拒绝为 invalid_occurred_at。

## Alternatives considered

### 使用 IP + User-Agent 推导 Visitor

拒绝。该方案不稳定、隐私风险高，无法可靠区分共享网络设备，也会把基础设施信息误当作用户身份。

### 客户端生成并直接信任 Session ID

拒绝。客户端时钟、多个标签页、清除存储和恶意输入会导致 Session 语义不一致。客户端可以提供辅助信息，但服务端必须能从 Raw Event 重建结果。

### 只按 received_at 做 Sessionization

拒绝。网络延迟和批量发送会改变用户真实行为顺序，也无法稳定处理迟到事件。行为指标使用 occurred_at，received_at 只用于接收诊断和 freshness。

### 只按 30 分钟 inactivity、不拆 UTC 午夜

拒绝作为当前默认规则。Phase 3/4 的报表已经以 UTC 日为边界，跨午夜不拆分会让每日 Session 报表难以解释。未来站点时区必须通过单独 contract 引入。

## Consequences

- Page View 统计保持向后兼容，Visitor/Session 可以在新字段逐步覆盖后启用。
- 迟到事件需要可重建的 Session 结果，不能只依赖不可逆的累计计数器。
- 24 小时窗口之外的迟到事件需要显式 backfill/rebuild；API 的 `data_as_of` 在 Phase 5 PR3 固定为所有参与聚合的共同 processed watermark。
- API 使用固定的日期/维度响应结构、20 默认 limit、100 最大 limit 和 aggregation_version。
- Browser SDK 需要第一方站点存储和随机 ID 生成能力，但不需要登录系统或跨站追踪。
- localStorage 方案不要求 Collector 或 Dashboard 读取浏览器存储；未来跨子域或服务端协作需要单独评估 Cookie。
- Phase 6 仍需要选择具体的 User-Agent parser、执行 additive migration，并实现 backfill/rebuild 命令；parser version、generation rollback 和 freshness contract 已在 Phase 5 PR3 冻结。
- 现阶段的 30 分钟、UTC 午夜和缺失 Visitor 语义已经冻结，后续实现不能隐式修改。

## Compatibility boundary

This decision is additive to the Phase 3/4 Page View workflow. Existing Protocol V1 events, `site_id + event_id` idempotency, UTC Page View dates, and current Overview/Reports contracts remain unchanged. Events without `visitor_id` continue to count as Page Views but are excluded from Visitor and Session metrics; they are never assigned a shared fallback identity.

The User-Agent parser implementation, versioning, upgrade policy, and historical reprocessing strategy remain a Phase 6 prerequisite. This ADR freezes the privacy and output boundary only: raw User-Agent data is not exposed through Analytics API or Dashboard, derived values must carry parser/version semantics, and unrecognized values map to `unknown`.
