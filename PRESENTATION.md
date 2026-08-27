<!-- Purpose: Stores presentation material for explaining FunnyFileManager. -->

---
marp: true
theme: default
paginate: true
---

# FunnyFileManager (ffm)
### A Modern Far Manager Clone in Rust

---

## What is ffm?

- **Dual-panel** terminal file manager
- Inspired by **Far Manager** and **Norton Commander**
- Written in **100% Rust**
- Powered by **Ratatui** TUI library

---

## Core Features

- **Side-by-side** view for productive file management
- **Instant navigation** with standard NC/Far hotkeys (F1-F10)
- **AWS S3 Integration**: Browse buckets like local drives
- **Quick Search**: Type letters to jump to files instantly
- **Multi-select**: Mark multiple files for batch operations
- **Cross-platform**: Linux, macOS, and Windows

---

## Architecture

- **Models**: Clean separation of state (Panels, Entries, App)
- **Handlers**: Modular logic for file operations and inputs
- **UI**: Declarative rendering using Ratatui widgets
- **Virtual Drives**: Extensible system for S3 and other backends

---

## Technology Stack

- **Rust**: Memory safety and performance
- **Ratatui**: The leading Rust TUI library
- **Crossterm**: Cross-platform terminal handling
- **AWS SDK**: Native S3 support
- **Tempfile**: Robust testing infrastructure

---

## Quality Assurance

- **83%+ Code Coverage**
- **Integration Tests** for all Far Manager hotkeys
- **Mutation Testing** with `cargo-mutants`
- **Tarpaulin` for coverage reporting

---

## Future Roadmap

- Archive support (.zip, .tar.gz)
- Syntax highlighting in viewer
- Plugin system (Lua or WASM)
- Built-in terminal emulator

---

## Get Started

```bash
cargo install --path .
ffm
```

**Thank you!**
