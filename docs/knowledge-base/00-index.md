<!-- Purpose: Indexes the topic-based knowledge base for agents and maintainers. -->

# 🧭 Knowledge Base Index

This is the structured knowledge base for FunnyFileManager, designed for use by LLMs (Claude Code, Gemini, Codex, etc.) via Retrieval-Augmented Generation (RAG).

## 🚀 How to Use This Knowledge Base

- Always start by reading this index.
- When you need information on a topic, read the specific module file(s).
- Prefer reading focused topic files over asking for large context dumps.
- The structure is topic-based to make retrieval efficient.

## 🏛️ Topic Modules

### 📖 01. Project Overview
- `01-project-overview/project-philosophy.md` — Core values and design philosophy
- `01-project-overview/far-manager-inspiration.md` — Relationship with Far Manager

### 🏛️ 02. Architecture
- `02-architecture/system-architecture.md` — Runtime architecture: layers, event loop, state model, subsystems, invariants
- `02-architecture/module-structure.md` — Detailed breakdown of every module

### ✅ 03. Rust Best Practices
- `03-rust-best-practices/type-system.md` — Enums, newtypes, making illegal states unrepresentable (**critical**)

### ✅ 04. Development Workflow
- `04-development-workflow/build-run-test.md` — Commands and environment setup
- `04-development-workflow/git-and-commits.md` — Commit conventions and workflow

### 🧪 05. Testing
- `05-testing/testing-strategy.md` — Overall testing philosophy and infrastructure

### 🎨 06. UI and Interaction
- `06-ui-and-interaction/overlays-and-modals.md` — The Overlay enum and modal system
- `06-ui-and-interaction/rendering-and-layout.md` — Ratatui layout and color scheme
- `06-ui-and-interaction/keyboard-handling.md` — Main key dispatch, quick search, popup priority

### ✨ 07. Core Features
- `07-core-features/file-operations.md` — Copy, move, delete, mkdir, unique names
- `07-core-features/s3-integration.md` — S3 client, and the tutorials under `docs/tutorials/`
- `07-core-features/command-line.md` — Built-in commands and shell fallback
- `07-core-features/apply-command.md` — Ctrl+G, Far substitutions, panel prefixes and selection lists
- `07-core-features/main-menu.md` — F9 operations, panel modes, options, and configuration
- `07-core-features/keepass-vault.md` — `.kdbx` browser, lock/unlock state machine, secret-handling rules

### 🌍 08. Platform Support
- `08-platform-support/windows-behavior.md`

### 🧑‍💻 09. Far Manager Compatibility
- `09-far-manager-compatibility/hotkeys-and-parity.md`

### 🚀 10. Agent Usage
- `10-agent-usage/how-to-use-this-knowledge-base.md` — Instructions for LLMs on using this RAG system

## 🧭 Quick Reference for Common Tasks

- **Starting work on the project** → Read `02-architecture/system-architecture.md`, then `02-architecture/module-structure.md`
- **Rust coding / refactoring** → Read `03-rust-best-practices/type-system.md` first
- **Writing tests** → Read `05-testing/testing-strategy.md`
- **Understanding UI behavior** → Read files in `06-ui-and-interaction/`

---

**This knowledge base is the single source of truth.** Agent instruction files (AGENTS.md, CLAUDE.md, GEMINI.md) should only contain high-level guidance on *how* to use this base, not duplicate its content.
