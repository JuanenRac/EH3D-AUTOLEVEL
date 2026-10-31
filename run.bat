@echo off
rem Runs eh3d-autolevel with its sample. Extra arguments replace the sample: run.bat [ARGUMENTS]
cd /d "%~dp0"
set "ARGS=%*"
if "%ARGS%"=="" set "ARGS=info examples/table.txt"
cargo run --release --quiet -- %ARGS%
if not defined NOPAUSE pause
exit /b %ERRORLEVEL%
