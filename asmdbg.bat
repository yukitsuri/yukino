@echo off
setlocal

where py >nul 2>&1
if %errorlevel% == 0 (
    py -3 "%~dp0scripts\build-debug.py" %*
) else (
    python "%~dp0scripts\build-debug.py" %*
)
exit /b %errorlevel%
