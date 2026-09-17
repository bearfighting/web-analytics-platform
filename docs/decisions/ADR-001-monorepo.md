# ADR-001：使用 pnpm Monorepo 并按阶段创建模块

- Status: Accepted
- Date: 2026-09-17

## Context

项目包含浏览器端 TypeScript、未来的 Rust Backend / Storage、Protocol 文件和 Dashboard。开发顺序是线性的，启动阶段不应创建没有实际内容的业务模块。

## Decision

使用 pnpm workspace 作为当前 Monorepo 基础。Phase 0 只将实际存在的 Next.js Playground 加入 workspace；`packages/`、`services/`、`crates/` 和 Dashboard 在对应阶段创建时再加入。

跨语言 Protocol 保存在独立的 `protocol/` 目录，不作为 pnpm workspace package。Rust workspace 等进入 Backend 阶段后再建立。

统一脚本负责本地和 CI 的安装后检查、测试、格式检查和构建流程。

## Consequences

- 新环境可以使用一套 pnpm 命令启动和验证当前项目。
- 目录结构不会被长期空模块占用。
- Protocol 保持独立于 TypeScript 和 Rust 实现。
- 后续加入 Rust workspace 时需要扩展统一脚本和 CI。
