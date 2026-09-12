# Transnet 约定

版本：1.0.1

本约定适用于整个仓库；新的版本化约定只有在更新仓库引用后才会取代本文件。

## Rust

- 遵循 Google Rust Style Guide，再遵循仓库配置。
- 使用两空格缩进、禁止 Tab、使用 `rustfmt.toml`，目标行宽 100 列。
- 每个 Rust 文件以 `//!` 模块注释开头；每个公开项目使用简洁的 `///` 契约说明。
- 可失败公开 API 添加 `# Errors`，不安全 API 添加 `# Safety`。
- 进程边界使用带上下文的 `anyhow`，库错误使用 `thiserror`；测试之外避免 `unwrap` 和 `expect`。
- 使用结构化 `tracing`；不得记录原文、学习者内容、凭据、能力令牌或供应商响应正文。
- 不得阻塞 Tokio 执行器线程。

## 仓库与文档

- Rust 位于 `src/`，集成测试位于 `tests/`，配置位于 `config/`，手写文档位于 `docs/`。
- 英文文档放在 `docs/`；每种本地化语言使用独立的 `docs_<language>/` 目录，例如中文使用 `docs_cn/`。
- 本地化目录中的每个内部文件都使用语言后缀，例如 `index_cn.md`、`port_cn.md`；其他语言使用对应后缀。
- 文件名应简短、描述性强且尽量唯一；禁止重复文档。
- 系统设计位于 `docs/transnet.md`，任务指南位于 `docs/guides/`，人类可读契约位于 `docs/interfaces/`，机器契约位于 `docs/reference/`。
- 每个契约只有一个权威归属；其他文档只摘要并链接，不复制正文。
- 段落和简单列表项保持单行；架构与流程使用 Mermaid。
- 请求和响应示例优先使用缩进 JSON；仅在确有语法高亮或多行非 JSON 内容价值时使用围栏代码块。
- 文档必须描述已实现行为，并随公开 API、配置或进程边界变化更新。
- 不提交生成文件、构建产物、日志、运行状态、凭据和编辑器状态；提交 `Cargo.lock`。

## 验证

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo doc --no-deps
```

## Git

- 使用 `feat:`、`fix:`、`docs:`、`refactor:`、`test:` 或 `chore:` 开头的 Conventional Commit。
- 分支使用意图前缀和 kebab-case 名称。
- 提交前检查状态和差异，只暂存明确路径，不提交无关修改。
