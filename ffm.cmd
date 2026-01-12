@echo off
REM Build and run FunnyFileManager on Windows
REM Usage: ffm.cmd [build|run|test|coverage|release]

if "%1"=="" goto run
if "%1"=="build" goto build
if "%1"=="run" goto run
if "%1"=="test" goto test
if "%1"=="coverage" goto coverage
if "%1"=="release" goto release
if "%1"=="help" goto help

echo Unknown command: %1
goto help

:build
echo Building FunnyFileManager...
cargo build
if errorlevel 1 (
    echo Build failed!
    exit /b 1
)
echo Build successful!
goto end

:run
echo Building and running FunnyFileManager...
cargo run
goto end

:test
echo Running tests...
cargo test --features test-helpers
goto end

:coverage
echo Generating test coverage report...
cargo llvm-cov --features test-helpers --html
echo Coverage report saved to target\llvm-cov\html\index.html
goto end

:release
echo Building release version...
cargo build --release
if errorlevel 1 (
    echo Release build failed!
    exit /b 1
)
echo Release build successful!
echo Executable: target\release\ffm.exe
goto end

:help
echo FunnyFileManager build script
echo.
echo Usage: ffm.cmd [command]
echo.
echo Commands:
echo   build     - Build debug version
echo   run       - Build and run (default)
echo   test      - Run all tests
echo   coverage  - Generate test coverage report
echo   release   - Build release version
echo   help      - Show this help message
echo.
goto end

:end