# Existing Shared Rust Test Helpers

This directory currently contains the shared helper module used by the existing top-level Cargo integration-test targets.

Future cleanup can migrate reusable harness code into `tests/support/`, but only together with explicit Cargo test-target/module changes. Keep the current location stable for now so test discovery is not accidentally broken.
