@echo off
call npm ci
if errorlevel 1 exit /b %ERRORLEVEL%

call npm run build
if errorlevel 1 exit /b %ERRORLEVEL%

if exist out\web rmdir /s /q out\web
if not exist out mkdir out
move src\web\dist out\web
if errorlevel 1 exit /b %ERRORLEVEL%

cargo test --lib --all-features --no-fail-fast
exit /b %ERRORLEVEL%
