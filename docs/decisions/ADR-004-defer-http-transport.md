# ADR-004：Backend 契约确定前延后 HTTP Transport

## 状态

已接受

## 决策

PR4 只实现 `analytics-browser` 内部的有界内存 Buffer、定时 flush、主动 flush 和本地 MockTransport，不实现 client 端 HTTP API request，也不创建独立 `transport` package。

## 原因

Backend 尚未确定 endpoint、认证、batch request/response、HTTP 错误、CORS 和重试语义。现在实现 FetchTransport 会提前固化这些边界，并可能在 Backend 阶段返工。当前 SDK 通过 `analytics-core` 的 Transport 接口验证完整事件 workflow，待 Backend API 契约稳定后再实现真实 Transport。
