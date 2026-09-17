# Next.js Router Playground

Router Playground 是 Phase 0 的行为实验场，只用于观察 Next.js App Router 导航，不生成正式 Analytics Event，也不实现 SDK、Observer、Session 或统计逻辑。

## 启动

Host 模式：

```bash
pnpm dev
```

Docker 模式：

```bash
pnpm docker:dev
```

两种方式都通过 `http://localhost:3000` 访问。

## 页面场景

| 页面                | 用途                 |
| ------------------- | -------------------- |
| `/`                 | 初始页面加载         |
| `/about`            | 普通页面导航         |
| `/products/example` | 动态路由             |
| `/search?q=test`    | Search params        |
| `/nested`           | 共享 layout 的父页面 |
| `/nested/child`     | 共享 layout 的子页面 |

页面底部的 Navigation Controls 提供：

- Next `<Link>` 导航
- `router.push()`
- `router.replace()`
- `router.back()` 和 `router.forward()`
- Search params 更新
- Hash 更新

## Navigation Debug Panel

Debug Panel 显示：

- 当前 URL
- pathname
- search params
- hash
- `document.title`
- 当前页面生命周期内的导航日志

日志包含 timestamp、previous URL、current URL 和 detected change。可观察的变化包括 initial、pathname、search params、push、replace、back / forward 和 hash。

Hash 变化在 Phase 0 只作为 Router 行为记录，不决定是否计为未来的 Page View。

## 扩展场景

新增场景时只修改 Playground 页面和导航控件，并同步更新本文件的场景表。正式 NavigationObserver 接口和 Client SDK 在 Phase 1 创建，不应在 Playground 中提前实现。
