#!/bin/bash
# Create Desktop Entry for FunnyFileManager on Linux

# Determine desktop directory
DESKTOP_DIR="$HOME/Desktop"
if [ ! -d "$DESKTOP_DIR" ]; then
    # Try common alternatives
    DESKTOP_DIR="$HOME/Рабочий стол"
    if [ ! -d "$DESKTOP_DIR" ]; then
        DESKTOP_DIR="$(xdg-user-dir DESKTOP 2>/dev/null || echo $HOME/Desktop)"
    fi
fi

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FFM_PATH="$SCRIPT_DIR/target/release/ffm"

# Check if FFM is built
if [ ! -f "$FFM_PATH" ]; then
    echo "FFM executable not found at $FFM_PATH"
    echo "Building FFM in release mode..."
    
    cd "$SCRIPT_DIR"
    cargo build --release
    
    if [ ! -f "$FFM_PATH" ]; then
        echo "Failed to build FFM. Please run 'cargo build --release' manually."
        exit 1
    fi
fi

# Create desktop entry
DESKTOP_FILE="$DESKTOP_DIR/funny-file-manager.desktop"

cat > "$DESKTOP_FILE" << EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Funny File Manager
Comment=Terminal-based file manager inspired by Far Manager
Exec=$FFM_PATH
Path=$SCRIPT_DIR
Icon=utilities-file-manager
Terminal=true
Categories=Utility;FileManager;
EOF

# Make it executable
chmod +x "$DESKTOP_FILE"
chmod +x "$FFM_PATH"

echo "Desktop entry created at: $DESKTOP_FILE"

# For systems that require desktop file validation
if command -v desktop-file-validate &> /dev/null; then
    desktop-file-validate "$DESKTOP_FILE"
fi

# Optional: Install to applications menu
APPLICATIONS_DIR="$HOME/.local/share/applications"
if [ -d "$APPLICATIONS_DIR" ]; then
    cp "$DESKTOP_FILE" "$APPLICATIONS_DIR/"
    echo "Application menu entry created at: $APPLICATIONS_DIR/funny-file-manager.desktop"
fi