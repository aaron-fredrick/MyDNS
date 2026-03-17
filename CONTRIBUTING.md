# Contributing to MyDNS

First off, thank you for considering contributing to MyDNS! It's people like you who make MyDNS such a great tool.

## Code of Conduct

By participating in this project, you agree to abide by the terms of our professional standards: respect the architecture, write clean code, and be helpful to others.

## How Can I Contribute?

### Reporting Bugs
* Check the [Issues](https://github.com/aaron-fredrick/MyDNS/issues) to see if it has already been reported.
* If not, open a new issue. Include a clear title, a description of the problem, and steps to reproduce.

### Suggesting Enhancements
* Open an issue with the [feature] tag.
* Describe the current behavior and what the new behavior should look like.

### Pull Requests
1. **Fork the repo** and create your branch from `main`.
2. **Coding Standards**:
    - All functions and methods must use `camelCase`.
    - Local variables must use `snake_case`.
    - Strictly follow **Command Query Separation (CQS)**. Separate data querying from side-effect commands.
3. **Tests**: Ensure all tests pass (`cargo test`). Add new tests for new features.
4. **Linting**: Ensure `cargo clippy` is happy.
5. **Documentation**: Update the README or inline docs if you change functionality.

## Project Structure

* `/src/dns`: Core DNS resolution engine.
* `/src/cache`: TTL-aware persistent caching.
* `/src/db`: SQLite migrations and record persistence.
* `/src/web`: Management dashboard (Axum + Vanilla JS).

## Development Environment
- Rust (Stable)
- SQLite3
- Windows or Linux

Thank you for your contributions!
