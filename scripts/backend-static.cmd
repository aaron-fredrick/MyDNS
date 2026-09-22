@echo off
cargo clippy --all-targets --all-features -- -D warnings
if errorlevel 1 exit /b %ERRORLEVEL%
cargo check --all-targets --all-features
exit /b %ERRORLEVEL%
