# Getting Started

## Requirements

- Node.js 22 LTS
- pnpm 11

Phase 0 不需要 Rust、Cargo、Docker、PostgreSQL 或其他后端依赖。

## Install

在仓库根目录执行：

```bash
pnpm install
```

## Run the Playground

```bash
pnpm dev
```

默认访问：

```text
http://localhost:3000
```

当前 Playground 只提供最小 Next.js App Router 页面。导航场景将在后续 PR 中加入。

## Check

```bash
pnpm check
```

执行 Prettier 格式检查、TypeScript 类型检查和 ESLint。

## Format

```bash
pnpm format
```

使用 Prettier 格式化项目文件。格式规则包括统一缩进、引号、分号、trailing comma 和文件末尾 newline。

Import 顺序由 ESLint `import/order` 检查，代码边界的空行由 ESLint padding 规则检查。新增 workspace 时，应提供同名的 `format` 和 `format:check` scripts，使根目录命令自动覆盖它。

## Test

```bash
pnpm test
```

当前还没有业务测试，命令会执行占位测试并正常结束。

## Build

```bash
pnpm build
```

构建 Next.js Playground 的生产版本。

## Scope

本阶段不包含：

- Analytics SDK
- Router Observer
- Event Protocol
- Docker Compose
- Backend 或数据库
