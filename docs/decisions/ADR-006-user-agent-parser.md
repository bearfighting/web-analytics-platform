# ADR-006：Phase 6 User-Agent Parser 采用 Rust 原生 Woothee

- Status: Accepted
- Date: 2026-09-21

## Context

Phase 6 需要从 Raw Event 中受限保留的 User-Agent 派生出低基数的 Device、Browser 和 OS 维度。Parser 结果必须能够带版本语义，并且升级时可以通过新 generation 重建和回滚。

Processor 是 Rust 服务。Phase 6 第一阶段不希望增加独立 Parser 服务、网络调用或额外运行时数据文件。

## Decision

使用 Rust crate `woothee`，版本锁定为 `0.13.0`。生产 parser version 使用：

```text
woothee-0.13.0
```

Processor 后续通过内部 adapter 隔离第三方 crate。Adapter 的稳定边界为：

```text
parse(user_agent, parser_version)
  → device, browser_family, browser_major, os_family, os_major
```

公开分类格式固定为：

- Device：`desktop`、`mobile`、`tablet` 或 `unknown`；
- Browser：`family` 或 `family:major`；
- OS：`family` 或 `family:major`。

无法识别时返回 `unknown`；无法稳定识别 major version 时只返回 family，不猜测版本。原始 User-Agent 不进入 API、Dashboard、结构化日志或 normalized context 之外的派生表。

Parser 升级必须创建新的 `parser_version` 和 generation。旧 generation 保留，升级结果通过 shadow parse/rebuild 对账后才能 active；回滚只切回旧 generation，不修改 Raw Event。

## Rationale

Woothee 提供直接的 Rust parser API，并将 parser dataset 作为 crate 的一部分使用，不需要额外加载 `regexes.yaml` 文件。`uaparser` 作为备选不采用，因为其典型使用方式要求应用提供并管理外部 YAML regex dataset，从而增加部署和数据版本管理边界。

## Consequences

- Cargo dependency 和 lockfile 必须锁定 `woothee 0.13.0`；
- PR1 锁定 dependency 和 lockfile，但不实现正式 Parser adapter；
- PR3 实现 adapter 时必须添加 canonical User-Agent fixtures；
- parser 结果属于 generation 输入，不能原地更新历史 normalized context；
- 未来升级 parser 必须新增 ADR 或更新本 ADR，并创建新的 parser generation。

## References

- [woothee Rust API](https://docs.rs/woothee/0.13.0/woothee/)
- [uaparser Rust API](https://docs.rs/uaparser/0.6.4/uaparser/)
