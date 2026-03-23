@echo off
echo Gentleman Guardian Angel - Installer (Windows)
echo.

set "SCRIPT_DIR=%~dp0"
set "INSTALL_DIR=%USERPROFILE%\bin"
set "LIB_INSTALL_DIR=%USERPROFILE%\bin\lib\gga"

echo Installing to: %INSTALL_DIR%
echo.

mkdir "%INSTALL_DIR%" 2>nul
mkdir "%LIB_INSTALL_DIR%" 2>nul

echo Copying files...
copy "%SCRIPT_DIR%bin\gga" "%LIB_INSTALL_DIR%\gga.sh" /y
copy "%SCRIPT_DIR%lib\providers.sh" "%LIB_INSTALL_DIR%\providers.sh" /y
copy "%SCRIPT_DIR%lib\cache.sh" "%LIB_INSTALL_DIR%\cache.sh" /y
copy "%SCRIPT_DIR%lib\pr_mode.sh" "%LIB_INSTALL_DIR%\pr_mode.sh" /y

echo Updating LIB_DIR...
set "BASH_LIB_DIR=%LIB_INSTALL_DIR:\=/%"
powershell -Command "(Get-Content '%LIB_INSTALL_DIR%\gga.sh') -replace 'LIB_DIR=.*', 'LIB_DIR=\"%BASH_LIB_DIR%\"' | Set-Content '%LIB_INSTALL_DIR%\gga.sh'"

echo Creating wrapper...
REM Find Git Bash path (prefer Git Bash over WSL)
set "BASH_PATH="
if exist "C:\Program Files\Git\usr\bin\bash.exe" (
    set "BASH_PATH=C:\Program Files\Git\usr\bin\bash.exe"
) else (
    set "BASH_PATH=bash"
)

(
    echo @echo off
    echo "%BASH_PATH%" "%LIB_INSTALL_DIR%\gga.sh" %%*
) > "%INSTALL_DIR%\gga.cmd"

echo.
echo Installation complete!
echo.
pause