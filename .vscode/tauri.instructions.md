# Rust Code Guidelines for Tauri Backend

- **Modular Design**: Organize code into small, focused modules and functions. Use separate files for major features or logical units.
- **Error Handling**: Prefer `Result<T, E>` and use crates like `anyhow` or `thiserror` for robust error management. Avoid panics in production code.
- **Type Safety**: Leverage Rust's strong typing. Use enums, structs, and traits to model data and behavior clearly.
- **Async & Performance**: Use `async`/`await` and libraries like `tokio` for non-blocking operations. Avoid blocking the main thread.
- **Documentation**: Write clear doc comments (`///`) for public functions, structs, and modules. Use `cargo doc` to generate docs.
- **Testing**: Add unit tests for core logic. Use `#[cfg(test)]` and the `test` module pattern.
- **Clippy & Formatting**: Run `cargo clippy` and `cargo fmt` regularly to enforce best practices and consistent style.
- **Safety & Security**: Avoid unsafe code unless absolutely necessary. Validate all external input.
- **Developer Experience**: Use clear naming, avoid magic numbers, and document complex logic. Prefer explicitness over cleverness.

_These guidelines help ensure your Tauri backend is robust, maintainable, and easy for any Rust developer to contribute to._
