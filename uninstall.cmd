@echo off
echo ============================================================
echo   Gentleman Guardian Angel - Uninstaller (Windows)
echo ============================================================
echo.

set "INSTALL_DIR=%USERPROFILE%\bin"
set "FOUND=false"

REM Check for gga.exe (Rust binary)
set "GGAEXE=%INSTALL_DIR%\gga.exe"
if exist "%GGAEXE%" (
    del "%GGAEXE%"
    echo Removed: %GGAEXE%
    set "FOUND=true"
)

REM Check for old bash wrapper files (legacy)
set "GGACMD=%INSTALL_DIR%\gga.cmd"
set "GGABAT=%INSTALL_DIR%\gga.bat"
if exist "%GGACMD%" (
    del "%GGACMD%"
    echo Removed: %GGACMD%
    set "FOUND=true"
)
if exist "%GGABAT%" (
    del "%GGABAT%"
    echo Removed: %GGABAT%
    set "FOUND=true"
)

REM Check lib directory (legacy bash files)
set "LIBDIR=%INSTALL_DIR%\lib\gga"
if exist "%LIBDIR%" (
    rmdir /s /q "%LIBDIR%"
    echo Removed: %LIBDIR%
    set "FOUND=true"
)

REM Check if bin directory is empty
if exist "%INSTALL_DIR%" (
    dir /b "%INSTALL_DIR%" | findstr . >nul
    if errorlevel 1 (
        rmdir "%INSTALL_DIR%"
        echo Removed empty directory: %INSTALL_DIR%
    )
)

REM Check global config (optional)
set "GLOBAL_CONFIG=%USERPROFILE%\.config\gga"
if exist "%GLOBAL_CONFIG%" (
    echo.
    set /p confirm="Remove global config (%GLOBAL_CONFIG%)? (y/N): "
    if /i "!confirm!"=="y" (
        rmdir /s /q "%GLOBAL_CONFIG%"
        echo Removed: %GLOBAL_CONFIG%
    )
)

if "%FOUND%"=="false" (
    echo gga was not found on this system
)

echo.
echo Note: Project-specific configs (.gga) and git hooks
echo       were not removed. Remove them manually if needed.
echo.
pause