@echo off
setlocal

REM Change to repository root (directory of this script)
pushd "%~dp0" >nul

REM Configuration and binary paths
set "CONFIG_PATH=%cd%\config\config.toml"
set "BINARY_PATH=%cd%\target\release\data_exporter.exe"

REM Optional arguments parsing (--skip-build / --no-build)
set "NO_BUILD=0"
set "ARGS="
:parse_args
if "%~1"=="" goto after_parse
if /I "%~1"=="--skip-build" (
    set "NO_BUILD=1"
) else (
    if /I "%~1"=="--no-build" (
        set "NO_BUILD=1"
    ) else (
        if defined ARGS (
            set "ARGS=%ARGS% %~1"
        ) else (
            set "ARGS=%~1"
        )
    )
)
shift
goto parse_args
:after_parse

if not exist "%CONFIG_PATH%" (
    echo [ERROR] Configuration file not found: %CONFIG_PATH%
    echo         Create it first - see docs/exporter-setup.md.
    popd >nul
    exit /b 1
)

if "%NO_BUILD%"=="0" goto build_release
if exist "%BINARY_PATH%" goto after_build

echo [WARN] Release binary not found. Building data_exporter...
cargo build --release
if errorlevel 1 (
    echo [ERROR] Cargo build failed. Aborting.
    popd >nul
    exit /b 1
)
goto after_build

:build_release
echo [INFO] Building data_exporter (release)...
cargo build --release
if errorlevel 1 (
    echo [ERROR] Cargo build failed. Aborting.
    popd >nul
    exit /b 1
)

:after_build

set "DATA_EXPORTER_CONFIG=%CONFIG_PATH%"

echo [INFO] Starting data_exporter with config "%CONFIG_PATH%"
if defined ARGS (
    "%BINARY_PATH%" %ARGS%
) else (
    "%BINARY_PATH%"
)
set "EXIT_CODE=%ERRORLEVEL%"

popd >nul
exit /b %EXIT_CODE%
