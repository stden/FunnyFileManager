#!/bin/bash
# Purpose: Launches FunnyFileManager from Unix shells.
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

# Install Colorer: the engine Far's FarColorer plugin uses, plus Far's schemes.
#
# Kept out of `cargo build` on purpose. Cloning and CMake-building a C++ library
# would turn every build of the file manager into something that needs the
# network and several minutes; this does it once, deliberately, and the build
# script then finds the result.
COLORER_HOME="${XDG_DATA_HOME:-$HOME/.local/share}/ffm"

install_colorer() {
    local lib="$COLORER_HOME/Colorer-library"
    local schemes="$COLORER_HOME/Colorer-schemes"

    if ! command -v cmake &> /dev/null; then
        print_error "cmake is required to build Colorer."
        echo "  Debian/Ubuntu: sudo apt-get install cmake g++ libicu-dev libxml2-dev zlib1g-dev libminizip-dev"
        exit 1
    fi

    mkdir -p "$COLORER_HOME"

    if [ -d "$lib/.git" ]; then
        print_status "Updating Colorer-library..."
        git -C "$lib" pull --ff-only --quiet || true
    else
        print_status "Cloning Colorer-library..."
        git clone --depth 1 https://github.com/colorer/Colorer-library "$lib" || exit 1
    fi

    print_status "Building Colorer-library (this takes a few minutes)..."
    cmake -S "$lib" -B "$lib/_build" -DCMAKE_BUILD_TYPE=Release \
        -DCOLORER_USE_VCPKG=OFF -DCOLORER_BUILD_TOOLS=OFF -DCOLORER_BUILD_TEST=OFF \
        > /dev/null || {
        print_error "CMake configure failed. Missing libicu-dev / libxml2-dev / zlib1g-dev / libminizip-dev?"
        exit 1
    }
    cmake --build "$lib/_build" --parallel || exit 1

    if [ -d "$schemes/.git" ]; then
        print_status "Updating the syntax schemes..."
        git -C "$schemes" pull --ff-only --quiet || true
    else
        print_status "Cloning the syntax schemes..."
        git clone --depth 1 https://github.com/colorer/Colorer-schemes "$schemes" || exit 1
    fi

    print_status "Colorer installed under $COLORER_HOME"
    print_status "Rebuild to pick it up:  ./ffm.sh install"
}

# Rebuild from source and install over whatever is already on PATH.
#
# --force is not optional here: without it `cargo install --path .` refuses to
# overwrite an install of the same version, so a rebuild after a source change
# would silently leave the OLD binary on PATH.
install_ffm() {
    print_status "Rebuilding and installing FunnyFileManager to ~/.cargo/bin..."
    cargo install --path . --force || { print_error "Install failed!"; exit 1; }

    local cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin/ffm"
    print_status "Installed: $cargo_bin"

    if ! command -v ffm &> /dev/null; then
        print_warning "~/.cargo/bin is not on your PATH — add it to use 'ffm' directly:"
        echo "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
        return
    fi

    # Refresh every OTHER ffm on PATH.
    #
    # A copy in /usr/local/bin outlived rebuild after rebuild and kept serving a
    # UI from August: `cargo install` writes only to ~/.cargo/bin, PATH order
    # alone decides which binary actually runs, and nothing warns you the two
    # disagree. Whichever copy wins should be the one just built.
    local other
    # `type -pa`, not `command -v -a`: only the former lists every match, and
    # every match is the point here.
    for other in $(type -pa ffm 2>/dev/null | grep -vFx "$cargo_bin"); do
        [ -f "$other" ] || continue
        cmp -s "$cargo_bin" "$other" && continue
        if cp "$cargo_bin" "$other" 2>/dev/null; then
            print_status "Refreshed stale copy: $other"
        else
            print_warning "Stale copy shadowing yours (not writable): $other"
            print_warning "  sudo cp $cargo_bin $other"
        fi
    done

    print_status "'ffm' resolves to: $(command -v ffm)"
    print_warning "Run 'hash -r' in shells opened before now — bash caches the old path."
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
        # Every invocation CI makes, because a feature-gated target is the one
        # that breaks unnoticed: cargo *skips* a test target whose
        # `required-features` are off rather than failing, so a plain
        # `cargo test` reports success while never compiling
        # `comprehensive_test` or either BDD suite.
        print_status "Running tests..."
        cargo test --features test-helpers || exit 1
        print_status "Running BDD suites..."
        cargo test --features test-helpers,s3 --test bdd --test bdd_ru || exit 1
        # `keepass` is enabled by default and covered above; retain this
        # focused invocation for a separate backend-test result.
        print_status "Running keepass feature tests..."
        cargo test --features keepass --lib keepass || exit 1
        ;;
    lint)
        check_rust
        print_status "Running Rust linter gate (fmt + clippy)..."
        bash "$SCRIPT_DIR/scripts/lint.sh" "${2:-all}"
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
    install|reinstall)
        check_rust
        install_ffm
        ;;
    colorer)
        install_colorer
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
        echo "  lint [mode]      Rust linter gate (fmt + clippy; see scripts/lint.sh)"
        echo "  coverage         Run test coverage analysis"
        echo "  clean            Clean build artifacts"
        echo "  colorer          Install Colorer + Far's syntax schemes, then rebuild"
        echo "  install          Rebuild and install FFM to ~/.cargo/bin (--force)"
        echo "  reinstall        Same as install"
        echo "  help             Show this help message"
        echo ""
        echo "Examples:"
        echo "  ./ffm.sh                  # Build and run in debug mode"
        echo "  ./ffm.sh release          # Build and run in release mode"
        echo "  ./ffm.sh build release    # Only build in release mode"
        echo "  ./ffm.sh test             # Run tests"
        echo "  ./ffm.sh lint             # fmt --check + clippy feature matrix"
        echo "  ./ffm.sh lint clippy-fast # all targets, default features"
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
