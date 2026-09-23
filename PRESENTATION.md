<!-- Purpose: Stores presentation material for explaining FunnyFileManager. -->

---
marp: true
theme: default
paginate: true
---

# 🗂️ FunnyFileManager (ffm)
### 🧑‍💻 A Modern Far Manager Clone in Rust

---

## 📖 What is ffm?

- **Dual-panel** terminal file manager
- Inspired by **Far Manager** and **Norton Commander**
- Written in **100% Rust**
- Powered by **Ratatui** TUI library

---

## ✨ Core Features

- **Side-by-side** view for productive file management
- **Instant navigation** with standard NC/Far hotkeys (F1-F10)
- **AWS S3 Integration**: Browse buckets like local drives
- **Quick Search**: Type letters to jump to files instantly
- **Multi-select**: Mark multiple files for batch operations
- **Cross-platform**: Linux, macOS, and Windows

---

## 🏛️ Architecture

- **Models**: Application state (panels, entries, one `Overlay`)
- **application**: Event loop and action effects
- **ops**: File, command, and dialog operations
- **ui**: Rendering with Ratatui. It does not run operations
- **handlers**: Compatibility re-exports, not the implementation

---

## 📚 Technology Stack

- **Rust**: Memory safety and performance
- **Ratatui**: The leading Rust TUI library
- **Crossterm**: Cross-platform terminal handling
- **AWS SDK**: Native S3 support
- **Tempfile**: Robust testing infrastructure

---

## ✅ Quality Assurance

- **Line coverage gate**: 98% of the measured root set, 75% for MCP (`cargo llvm-cov` in CI)
- **Integration tests** for hotkeys, menus, and file operations
- **Targeted mutation testing** with `cargo-mutants`

---

## ✨ Also in the tree

- Built-in editor (F4), syntax highlighting, and optional language servers
- tar and zip archive dialogs
- KeePass vault browser (`keepass` feature)
- MCP server for agents (`mcp/`)
- Console transcript (`Ctrl+O`), separate from an embedded shell

---

## 📦 Get Started

```bash
cargo install --path .
ffm
```

**Thank you!**
