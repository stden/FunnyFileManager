# FunnyFileManager (ffm)

Far Manager style dual-panel file manager for the terminal, written in Rust.

## Installation

```bash
cargo install --path .
```

This installs `ffm` to `~/.cargo/bin/`.

## Usage

```bash
ffm
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| **F1** | Help |
| **F3** | View file |
| **F4** | Edit file ($EDITOR or nano) |
| **F5** | Copy to other panel |
| **F6** | Move/Rename to other panel |
| **F7** | Create directory |
| **F8** | Delete |
| **F10** / **Q** | Quit |
| **Tab** | Switch panels |
| **↑↓** | Navigate |
| **←→** | Select panel |
| **Enter** | Open directory |
| **Esc** | Close overlay |

## Interface

```
┌─ /Users/user/Documents ─────────────┐┌─ /Users/user/Downloads ────────────┐
│» 📁 ..                       <DIR>  ││  📁 ..                       <DIR> │
│  📁 Projects                 <DIR>  ││  📁 Archives                 <DIR> │
│  📄 notes.txt               1.2 KB  ││  📄 file.zip               15.3 MB │
└─────────────────────────────────────┘└────────────────────────────────────┘
 ↑↓: Navigate | Enter: Open | Tab: Switch Panel | notes.txt | 1.2 KB
1Help  2UserMn3View  4Edit  5Copy  6RenMov7MkFold8Delete9ConfMn10Quit
```

## Building

```bash
cargo build --release    # Build
cargo test               # Run tests
cargo tarpaulin          # Coverage report
```

## Project Structure

```
src/
├── main.rs      # Entry point, event loop
├── models.rs    # Data structures
├── ui.rs        # UI rendering
├── handlers.rs  # File operations
└── utils.rs     # Utilities
```

## License

MIT
