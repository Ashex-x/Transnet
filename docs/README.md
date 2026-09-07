# Transnet documentation

Start here for project documentation. The tree separates system-wide design, task-oriented guides, and code-facing service reference.

## System design

- [Architecture](architecture.md): processes, request flow, ownership boundaries, and current limitations.
- [Coding conventions](../conventions.md): source, documentation, testing, and Git requirements.

## Guides

- [Development and operations](guides/development.md): prerequisites, workspace checks, local startup, and deployment safeguards.
- [Configuration](guides/configuration.md): active core TOML settings, gateway environment variables, and deferred configuration.

## Service reference

- [Service reference index](reference/README.md): choose between the core and gateway contracts.
- [Translation core](reference/core/README.md): core modules, HTTP API, provider integration, prompts, formats, and shared types.
- [Gateway](reference/gateway/README.md): gateway design and JSON API boundary.

```text
docs/
├── README.md                 # this index
├── architecture.md          # system-wide design
├── guides/                  # task-oriented instructions
│   ├── configuration.md
│   └── development.md
└── reference/               # code-facing contracts and module notes
    ├── README.md
    ├── core/
    └── gateway/
```

Documentation must describe implemented behavior. Planned capabilities belong in [`island-transnet/TODO.md`](../island-transnet/TODO.md) and must be labeled as deferred.
