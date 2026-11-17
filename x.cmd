@REM Purpose: Start the Windows FFM CPU profiling launcher beside this script.
@echo off
setlocal
where python >nul 2>&1
if not errorlevel 1 goto run_python
where py >nul 2>&1
if not errorlevel 1 goto run_py
echo Required command is missing: python 1>&2
exit /b 1

:run_python
python "%~dp0scripts\profile_windows.py" %*
exit /b %ERRORLEVEL%

:run_py
py -3 "%~dp0scripts\profile_windows.py" %*
exit /b %ERRORLEVEL%
