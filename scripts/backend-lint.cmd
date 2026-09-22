@echo off
cargo fmt --all -- --check
if errorlevel 1 exit /b %ERRORLEVEL%
cargo clippy --all-targets --all-features -- -D warnings
exit /b %ERRORLEVEL%
