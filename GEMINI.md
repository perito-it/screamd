## Purpose
read README.md to understand how this software works

## Architecture

`screamd` is implemented as a Rust service that transitions through distinct operational phases: Warning, Reboot, and Shutdown. The core logic resides in `src/service_core.rs`, which orchestrates these phases based on a `Config` loaded from `config.toml` and persistent `State` stored in `state.json`. Operating system interactions (e.g., setting banners, rebooting, shutting down) are abstracted through the `OsControl` trait defined in `src/os_control.rs`, with platform-specific implementations found in `src/linux_control.rs` and `src/windows_control.rs`. Asynchronous operations are handled using the `tokio` runtime.

## Workflow after making changes

After making any changes to the codebase, please run the following commands in order to ensure code quality and correctness:

1.  `cargo fmt` - To format the code.
2.  `cargo clippy -- -D warnings` - To catch common mistakes and treat warnings as errors.
3.  `cargo test` - To run all tests.

## Security Checks

Regularly run the following commands to check for security issues:

*   `cargo audit` - Checks for known vulnerabilities in dependencies.
*   `cargo geiger` - Scans for `unsafe` Rust code.
