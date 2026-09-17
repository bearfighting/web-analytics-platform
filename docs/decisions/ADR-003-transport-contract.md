# ADR-003：将 Transport Contract 放在 Analytics Core

## 状态

已接受

## 决策

`Transport` 只在 `analytics-core` 定义接口，Browser SDK 通过依赖注入使用它。具体 Fetch、Beacon、批量和重试实现留在后续独立 package。

## 原因

这样可以让事件管线保持 framework-agnostic，并用 Mock Transport 在 Node 环境验证完整 workflow，同时避免 PR3 提前引入网络、缓存或后端耦合。
