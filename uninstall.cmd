@echo off
setlocal enabledelayedexpansion

echo.
echo ============================================================
echo   Gentleman Guardian Angel - Uninstaller
echo ============================================================
echo.

set "FOUND=false"

REM Check for gga.cmd
set "GGACMD=%USERPROFILE%\bin\gga.cmd"
if exist "%GGACMD%" (
    del "%GGACMD%"
    echo Removed: %GGACMD%
    set "FOUND=true"
)

REM Check lib directory
set "LIBDIR=%USERPROFILE%\bin\lib\gga"
if exist "%LIBDIR%" (
    rmdir /s /q "%LIBDIR%"
    echo Removed: %LIBDIR%
    set "FOUND=true"
)

REM Check if bin directory is empty
set "BINDIR=%USERPROFILE%\bin"
if exist "%BINDIR%" (
    dir /b "%BINDIR%" | findstr . >nul
    if errorlevel 1 (
        rmdir "%BINDIR%"
        echo Removed empty directory: %BINDIR%
    )
)

REM Check global config (optional)
set "GLOBAL_CONFIG=%USERPROFILE%\.config\gga"
if exist "%GLOBAL_CONFIG%" (
    echo.
    echo Global config found at: %GLOBAL_CONFIG%
    echo To remove it manually, run:
    echo   rmdir /s /q "%GLOBAL_CONFIG%"
)

if "%FOUND%"=="false" (
    echo gga was not found on this system
)

echo.
echo Note: Project-specific configs (.gga) and git hooks
echo       were not removed. Remove them manually if needed.
echo.
pause