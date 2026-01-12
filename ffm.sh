#!/bin/bash
# Build and run FunnyFileManager on Unix-like systems (Ubuntu, macOS, etc.)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Function to print colored output
print_status() {
    echo -e "${GREEN}[FFM]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Check if Rust is installed
check_rust() {
    if ! command -v cargo &> /dev/null; then
        print_error "Rust is not installed!"
        echo "Please install Rust from https://rustup.rs/"
        echo "Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
}

# Build FFM
build_ffm() {
    print_status "Building FunnyFileManager..."
    
    if [ "$1" == "release" ]; then
        cargo build --release
        FFM_BINARY="target/release/ffm"
    else
        cargo build
        FFM_BINARY="target/debug/ffm"
    fi
    
    if [ $? -ne 0 ]; then
        print_error "Build failed!"
        exit 1
    fi
    
    print_status "Build completed successfully!"
}

# Run FFM
run_ffm() {
    if [ ! -f "$FFM_BINARY" ]; then
        print_warning "FFM binary not found, building..."
        build_ffm "$BUILD_MODE"
    fi
    
    print_status "Starting FunnyFileManager..."
    exec "$FFM_BINARY" "$@"
}

# Parse command line arguments
case "$1" in
    build)
        check_rust
        BUILD_MODE="${2:-debug}"
        build_ffm "$BUILD_MODE"
        ;;
    release)
        check_rust
        build_ffm "release"
        run_ffm "${@:2}"
        ;;
    run)
        BUILD_MODE="debug"
        FFM_BINARY="target/debug/ffm"
        run_ffm "${@:2}"
        ;;
    test)
        check_rust
        print_status "Running tests..."
        cargo test
        ;;
    coverage)
        check_rust
        print_status "Running test coverage..."
        if command -v cargo-tarpaulin &> /dev/null; then
            cargo tarpaulin
        else
            print_warning "cargo-tarpaulin not installed. Installing..."
            cargo install cargo-tarpaulin
            cargo tarpaulin
        fi
        ;;
    clean)
        print_status "Cleaning build artifacts..."
        cargo clean
        ;;
    install)
        check_rust
        print_status "Installing FunnyFileManager to ~/.cargo/bin..."
        cargo install --path .
        print_status "Installation complete! Make sure ~/.cargo/bin is in your PATH"
        ;;
    help|--help|-h)
        echo "FunnyFileManager Build & Run Script"
        echo ""
        echo "Usage: ./ffm.sh [command] [options]"
        echo ""
        echo "Commands:"
        echo "  (no command)     Build (if needed) and run FFM in debug mode"
        echo "  build [mode]     Build FFM (mode: debug|release, default: debug)"
        echo "  release          Build in release mode and run"
        echo "  run              Run FFM (assumes already built)"
        echo "  test             Run all tests"
        echo "  coverage         Run test coverage analysis"
        echo "  clean            Clean build artifacts"
        echo "  install          Install FFM to ~/.cargo/bin"
        echo "  help             Show this help message"
        echo ""
        echo "Examples:"
        echo "  ./ffm.sh                  # Build and run in debug mode"
        echo "  ./ffm.sh release          # Build and run in release mode"
        echo "  ./ffm.sh build release    # Only build in release mode"
        echo "  ./ffm.sh test             # Run tests"
        ;;
    *)
        # Default: build and run in debug mode
        check_rust
        BUILD_MODE="debug"
        FFM_BINARY="target/debug/ffm"
        
        if [ ! -f "$FFM_BINARY" ]; then
            build_ffm "debug"
        fi
        
        run_ffm "$@"
        ;;
esac