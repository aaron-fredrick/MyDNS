@echo off
call npm --prefix src/frontend ci
if errorlevel 1 exit /b %ERRORLEVEL%

call npm --prefix src/frontend run build
if errorlevel 1 exit /b %ERRORLEVEL%

cargo test --lib --all-features --no-fail-fast
exit /b %ERRORLEVEL%
