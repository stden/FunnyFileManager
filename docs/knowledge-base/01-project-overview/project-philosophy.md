<!-- Purpose: States the product and engineering philosophy behind FunnyFileManager. -->

# 📖 Project Philosophy

FunnyFileManager (FFM) is a terminal-based dual-panel file manager written in Rust, strongly inspired by Far Manager.

## 🏛️ Core Design Goals

- **Far Manager UX fidelity**: Reproduce the classic Far Manager feel (dark blue background RGB(0,0,128), F-key command bar, dual panels, overlays, quick search, etc.).
- **Modern Rust engineering**: Leverage Rust's type system aggressively (enums for state, newtypes for domain values) to make illegal states unrepresentable.
- **High testability**: The application is designed from the ground up to be thoroughly testable, including mutation testing and high coverage expectations.
- **Lightweight but powerful**: Keep the core small and focused while supporting powerful features like S3 integration and external editor support.
- **Agent / LLM friendly**: The project maintains a structured knowledge base (`docs/knowledge-base/`) so that AI coding agents can work effectively without massive context duplication.

## 🏛️ Key Architectural Principles

- Thin `main.rs` — all real logic lives in the library crate.
- Single `Overlay` enum as the source of truth for all modal states (instead of many scattered `bool`/`Option` fields).
- Heavy use of feature flags (`test-helpers`, `s3`) to control visibility of code and tests.
- Pure functions extracted into `utils.rs` wherever possible.
- Command line and keyboard handling designed for exhaustive, type-safe dispatch (see `Command` enum).

## 🚫 Non-Goals

- Not trying to be a full graphical file manager.
- Not aiming for maximum feature parity with every Far plugin.
- F4 is a built-in text editor with syntax highlighting and optional language servers. Alt+F4 still opens an external editor. The editor is not a second application with its own file manager.

The project values correctness, maintainability, and excellent developer/agent experience over raw feature count.
