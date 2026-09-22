@echo off
cargo fmt --all -- --check
exit /b %ERRORLEVEL%
