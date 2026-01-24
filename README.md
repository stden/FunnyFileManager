# FunnyFileManager
Far Manager interface implementation in Rust

## Description

A terminal-based file manager with a dual-panel interface inspired by Far Manager and Norton Commander. Built using Rust with the `ratatui` TUI library.

## Features

- **Dual Panel Interface**: Two side-by-side panels for easy file operations
- **Directory Navigation**: Navigate through directories with arrow keys
- **Visual Indicators**: Folders and files displayed with icons and color coding
- **File Information**: Shows file sizes and directory indicators
- **Keyboard-Driven**: Fast navigation using keyboard shortcuts
- **Cross-Platform**: Works on Linux, macOS, and Windows

## Requirements

- Rust 1.70 or later
- Cargo (Rust package manager)

## Installation

```bash
# Clone the repository
git clone https://github.com/stden/FunnyFileManager.git
cd FunnyFileManager

# Build the project
cargo build --release

# Run the application
cargo run
```

## Usage

### Keyboard Shortcuts

- **↑/↓ Arrow Keys**: Navigate up/down in the current panel
- **Home/End**: Jump to first/last entry
- **Enter**: Open selected directory
- **Backspace**: Go to parent directory
- **Tab**: Switch active panel
- **←/→ Arrow Keys**: Select left or right panel directly
- **F1**: Help
- **F3**: View file (read-only)
- **F4**: Edit file (`$EDITOR` or `nano`)
- **F5**: Copy (to other panel)
- **F6**: Move/Rename (to other panel)
- **F7**: Create directory
- **F8**: Delete
- **F10** or **Q**: Quit
- **F11**: AWS explorer (requires `aws` CLI + credentials)
- **Ctrl+R**: Refresh panels
- **Ctrl+U**: Swap panels
- **Ctrl+C**: Force quit

### Interface Layout

```
┌─ Left Panel ────────────────┐┌─ Right Panel ───────────────┐
│ 📁 ..              <DIR>    ││ 📁 ..              <DIR>    │
│ 📁 src             <DIR>    ││ 📁 src             <DIR>    │
│ 📄 Cargo.toml      125 B    ││ 📄 Cargo.toml      125 B    │
└─────────────────────────────┘└─────────────────────────────┘
┌─ Status Bar ───────────────────────────────────────────────┐
│ ↑↓: Navigate | Enter: Open | Tab: Switch | Q: Quit        │
└────────────────────────────────────────────────────────────┘
```

## Architecture

The application is structured with the following components:

- **Panel**: Manages directory state and file listings
- **App**: Coordinates both panels and handles active panel state
- **DirEntry**: Represents individual files and directories
- **UI Rendering**: Uses ratatui widgets for terminal rendering

## Building from Source

```bash
cargo build --release
```

The compiled binary will be available at `target/release/funny-file-manager`.

## Development

```bash
# Run in development mode
cargo run

# Run with debug output
RUST_LOG=debug cargo run

# Run tests (if available)
cargo test
```

## EKS / k9s helpers (optional)

If you use Amazon EKS with `k9s`, you can source `scripts/k9s-eks.sh` to get helper functions that update your kubeconfig and launch `k9s` with the right context:

```bash
source ./scripts/k9s-eks.sh

# Open k9s for a specific cluster
k9s_eks my-cluster us-west-2 myprofile

# Or fuzzy-pick a cluster (requires fzf)
k9s_eks_select us-west-2 myprofile
```

## Dependencies

- `ratatui` (0.26): Terminal UI library
- `crossterm` (0.27): Cross-platform terminal manipulation

## License

See LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues.

## Future Enhancements

- File copy/move/delete operations
- File search functionality
- Archive support (zip, tar, etc.)
- Syntax highlighting for file preview
- Configuration file support
- Custom key bindings
- Multiple color themes
