@echo off
echo ============================================================
echo   Gentleman Guardian Angel - Installer (Windows)
echo ============================================================
echo.

set "SCRIPT_DIR=%~dp0"
set "INSTALL_DIR=%USERPROFILE%\bin"

echo Install directory: %INSTALL_DIR%
echo.

REM Check if Rust binary exists
if not exist "%SCRIPT_DIR%gga-rust\target\release\gga.exe" (
    echo Error: Rust binary not found at gga-rust\target\release\gga.exe
    echo.
    echo Please build the Rust binary first:
    echo   cd gga-rust
    echo   cargo build --release
    echo.
    pause
    exit /b 1
)

REM Create install directory
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"

echo Copying gga.exe to %INSTALL_DIR%...
copy "%SCRIPT_DIR%gga-rust\target\release\gga.exe" "%INSTALL_DIR%\gga.exe" /y

echo.
echo Installation complete!
echo.
echo gga.exe is now available at: %INSTALL_DIR%\gga.exe
echo.

REM Check if install dir is in PATH
echo %PATH% | findstr /i /c:"%INSTALL_DIR%" >nul 2>&1
if errorlevel 1 (
    echo %INSTALL_DIR% is not in your PATH
    echo.
    echo To add it to your PATH permanently:
    echo   1. Open System Properties
    echo   2. Go to Advanced tab
    echo   3. Click Environment Variables
    echo   4. Edit PATH variable
    echo   5. Add: %INSTALL_DIR%
    echo.
) else (
    echo %INSTALL_DIR% is already in your PATH
    echo.
)

echo Getting started:
echo   1. Navigate to your project
echo   2. gga init
echo   3. Edit .gga config file
echo   4. gga install
echo   5. gga run
echo.
pause