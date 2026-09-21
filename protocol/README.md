# Protocol and Contract Layout

This directory is organized by stable contract meaning and version, not by
the project phase in which a contract was introduced.

```text
events/<version>/       Event envelope schemas, examples, and fixtures
contexts/<version>/     Versioned nested event context schemas
contracts/<service>/    HTTP and Analytics API service contracts
scenarios/<domain>/      Cross-service semantic scenarios
```

Event and context schemas are the cross-language source of truth. Service
contracts describe transport or query behavior, while scenarios describe
processing semantics that are shared by more than one implementation.

Do not add `phase-*` directories under `protocol/`. Project phases belong in
design and roadmap documents; protocol paths should remain stable when the
implementation roadmap advances.
