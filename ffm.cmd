@REM Purpose: Launches FunnyFileManager from Windows command prompts.
@echo off
setlocal
REM Build and run FunnyFileManager on Windows
REM Usage: ffm.cmd [build|run|test|coverage|release]
REM Compiler caching is opt-in, so this behaves exactly like plain Cargo.
set "FFM_CARGO_COMMAND=cargo"

if "%1"=="" goto run
if "%1"=="build" goto build
if "%1"=="run" goto run
if "%1"=="test" goto test
if "%1"=="coverage" goto coverage
if "%1"=="release" goto release
if "%1"=="colorer" goto colorer
if "%1"=="help" goto help

echo Unknown command: %1
goto help

:build
echo Building FunnyFileManager...
%FFM_CARGO_COMMAND% build
if errorlevel 1 (
    echo Build failed!
    exit /b 1
)
echo Build successful!
goto end

:run
REM Release by default: on Windows this launcher is how the file manager is
REM USED, not developed, and a dev-profile TUI redraws visibly slower. Cargo
REM has no default-profile setting (build.profile is not a config key), so
REM the launcher is the only place a Windows default can live.
echo Building and running FunnyFileManager (release)...
%FFM_CARGO_COMMAND% run --release
if errorlevel 1 exit /b 1
goto end

:test
echo Running tests...
%FFM_CARGO_COMMAND% test --features test-helpers
if errorlevel 1 exit /b 1
REM keepass is off by default, so the run above does not even compile
REM src\keepass.rs - its tests need their own invocation.
echo Running keepass feature tests...
%FFM_CARGO_COMMAND% test --features test-helpers,keepass --lib keepass
if errorlevel 1 exit /b 1
goto end

:coverage
REM cargo-llvm-cov cannot run the empty rustc-wrapper in .cargo/config.toml (it
REM executes " rustc"), and cmd cannot set an empty variable to override it, so
REM hand it a pass-through wrapper: env.exe, as CI does on Linux.
if defined RUSTC_WRAPPER goto coverage_run
if defined CARGO_BUILD_RUSTC_WRAPPER goto coverage_run
for /f "delims=" %%E in ('where env.exe 2^>nul') do if not defined CARGO_BUILD_RUSTC_WRAPPER set "CARGO_BUILD_RUSTC_WRAPPER=%%E"
if not defined CARGO_BUILD_RUSTC_WRAPPER if exist "%ProgramFiles%\Git\usr\bin\env.exe" set "CARGO_BUILD_RUSTC_WRAPPER=%ProgramFiles%\Git\usr\bin\env.exe"
if defined CARGO_BUILD_RUSTC_WRAPPER goto coverage_run
echo cargo llvm-cov needs a pass-through compiler wrapper and none was found.
echo Install Git for Windows, which ships usr\bin\env.exe, or set CARGO_BUILD_RUSTC_WRAPPER.
exit /b 1

:coverage_run
REM Git's env.exe must pass rustc's arguments through untouched.
set "MSYS_NO_PATHCONV=1"
echo Generating test coverage report...
%FFM_CARGO_COMMAND% llvm-cov --features test-helpers --html
if errorlevel 1 exit /b 1
echo Coverage report saved to target\llvm-cov\html\index.html
goto end

:release
echo Building release version...
%FFM_CARGO_COMMAND% build --release
if errorlevel 1 (
    echo Release build failed!
    exit /b 1
)
echo Release build successful!
echo Executable: target\release\ffm.exe
goto end

:colorer
REM Install Colorer: the engine Far's FarColorer plugin uses, plus Far's schemes.
REM
REM Kept out of `cargo build` on purpose. Cloning and CMake-building a C++
REM library would turn every build into something that needs the network and
REM several minutes; this does it once and the build script then finds it.
REM
REM Same layout as `ffm.py colorer`, so colorer-sys and the running program
REM look in one place on both platforms.
set "COLORER_HOME=%USERPROFILE%\.local\share\ffm"
set "COLORER_LIB=%COLORER_HOME%\Colorer-library"
set "COLORER_SCHEMES=%COLORER_HOME%\Colorer-schemes"

where cmake >nul 2>&1
if errorlevel 1 (
    echo cmake is required to build Colorer. Install it from https://cmake.org/download/
    exit /b 1
)
where git >nul 2>&1
if errorlevel 1 (
    echo git is required to fetch Colorer.
    exit /b 1
)

if not exist "%COLORER_HOME%" mkdir "%COLORER_HOME%"

if exist "%COLORER_LIB%\.git" (
    echo Updating Colorer-library...
    git -C "%COLORER_LIB%" pull --ff-only --quiet
) else (
    echo Cloning Colorer-library...
    git clone --depth 1 https://github.com/colorer/Colorer-library "%COLORER_LIB%"
    if errorlevel 1 exit /b 1
)

REM vcpkg stays ON here, unlike the Linux path: Windows has no system ICU,
REM libxml2 or zlib for CMake to find, so Colorer fetches its own.
echo Building Colorer-library ^(this takes a few minutes^)...
cmake -S "%COLORER_LIB%" -B "%COLORER_LIB%\_build" -DCOLORER_BUILD_TOOLS=OFF -DCOLORER_BUILD_TEST=OFF
if errorlevel 1 (
    echo CMake configure failed.
    exit /b 1
)
cmake --build "%COLORER_LIB%\_build" --config Release --parallel
if errorlevel 1 exit /b 1

if exist "%COLORER_SCHEMES%\.git" (
    echo Updating the syntax schemes...
    git -C "%COLORER_SCHEMES%" pull --ff-only --quiet
) else (
    echo Cloning the syntax schemes...
    git clone --depth 1 https://github.com/colorer/Colorer-schemes "%COLORER_SCHEMES%"
    if errorlevel 1 exit /b 1
)

echo.
echo Colorer installed under %COLORER_HOME%
echo Rebuild to pick it up:  ffm.cmd build
echo.
echo The build script links the vcpkg libraries from this build by itself.
echo If your toolchain names them differently, override the list:
echo   set COLORER_LINK_LIBS=name1,name2,...
goto end

:help
echo FunnyFileManager build script
echo.
echo Usage: ffm.cmd [command]
echo.
echo Commands:
echo   build     - Build debug version
echo   run       - Build and run the release profile (default)
echo   test      - Run all tests
echo   coverage  - Generate test coverage report
echo   release   - Build release version
echo   colorer   - Install the Colorer engine and Far's syntax schemes
echo   help      - Show this help message
echo.
goto end

:end
endlocal
exit /b 0
