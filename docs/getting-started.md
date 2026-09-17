# Getting Started

## Requirements

- Node.js 22 LTS
- pnpm 11

Phase 0 不需要 Rust、Cargo、PostgreSQL 或其他后端依赖。Docker 是可选的 Playground 开发方式。

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

当前 Playground 提供常见 Next.js App Router 导航场景和 Navigation Debug Panel。

可测试页面：

```text
/
/about
/products/example
/search?q=test
/nested
/nested/child
```

可测试操作：

```text
<Link>
router.push()
router.replace()
router.back()
router.forward()
search params
hash
```

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

`pnpm test` 同时校验 Event Protocol V1 的合法和非法 fixtures。

单独运行 Protocol 校验：

```bash
pnpm protocol:validate
```

## Build

```bash
pnpm build
```

构建当前所有 TypeScript packages，再构建 Next.js Playground 的生产版本。

## Docker Development

需要 Docker 和 Docker Compose v2：

```bash
pnpm docker:dev
```

该命令会构建并启动 Router Playground，访问地址仍为：

```text
http://localhost:3000
```

PR2 的 Compose 只运行 Playground，不包含 PostgreSQL 或其他后端服务。

## CI

GitHub Actions 会复用本地检查命令，并额外验证 Docker Compose 配置。CI 不构建或启动 Docker 镜像。

Phase 0 没有必需的环境变量；`.env.example` 仅用于说明未来配置的预留位置。

## 当前范围

当前已包含：

- `observer-core` 的通用导航契约
- `observer-next` 的 Next.js App Router Adapter
- `analytics-core` 的基础事件管线
- Next.js Router Playground

当前仍不包含：

- Browser SDK runtime 和 Browser Context
- Transport、Buffer 和网络发送
- Backend、Storage、数据库或 Dashboard
