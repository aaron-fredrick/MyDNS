@echo off
cargo test --lib --all-features --no-fail-fast
exit /b %ERRORLEVEL%
