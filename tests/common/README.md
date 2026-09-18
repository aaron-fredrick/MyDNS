# Legacy Shared Test Helpers

This directory currently contains the shared Rust integration-test module used by the existing top-level Cargo test targets.

Future cleanup can move reusable harness code into `tests/support/` once the Rust test entry points are migrated deliberately. Keep this directory stable for now so the current integration-test compilation model is not accidentally broken.
