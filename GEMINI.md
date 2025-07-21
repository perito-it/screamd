always build and run tests after modifications

## Workflow after making changes

After making any changes to the codebase, please run the following commands in order to ensure code quality and correctness:

1.  `cargo fmt` - To format the code.
2.  `cargo clippy -- -D warnings` - To catch common mistakes and treat warnings as errors.
3.  `cargo test` - To run all tests.
4.  `./install.sh build` - To test the release build

## Dependencies
*   Avoid introducing new dependencies unless absolutely necessary
*   When introducing a new dependency, explain the reason

## Security Checks

Regularly run the following commands to check for security issues:

*   `cargo audit` - Checks for known vulnerabilities in dependencies.
*   `cargo geiger` - Scans for `unsafe` Rust code.

## Key Technologies/Dependencies

This project utilizes the following key Rust crates:

*   `anyhow`: For flexible and ergonomic error handling.
*   `async-trait`: Enables `async` functions in traits.
*   `chrono`: For date and time manipulation.
*   `serde`: A framework for serializing and deserializing Rust data structures efficiently and generically.
*   `serde_json`: JSON support for `serde`.
*   `tempfile`: For creating temporary files and directories.
*   `tokio`: A runtime for writing asynchronous applications with Rust.
*   `toml`: For parsing and serializing TOML configuration files.
*   `winreg` (Windows-specific): For interacting with the Windows Registry.

## Testing Strategy

Tests in this project are primarily unit tests co-located with the code in `src/` modules. They are executed using `cargo test`.
