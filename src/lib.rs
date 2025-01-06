/*!
Cell types that can be sent/shared between threads, even if the underlying data cannot.

![logo](art/logo.png)

This crate may be considered an alternative to [fragile](https://crates.io/crates/fragile) crate,
but this is a bit more ergonomic in my opinion, and it also provides unsafe "hold my beer" variants.

# Safe wrappers

This crate provides runtime-checked wrappers, allowing you to temporarily send or share the underlying data,
but performs runtime checks to ensure that the data is only accessed on the correct thread.

# Unsafe wrappers

This crate also provides unchecked wrappers that allow you to send or share the underlying data
without any runtime checks (but with unsafe blocks).

This is useful in cases where you know that the data is not actually sent/shared between threads,
but Rust does not.  For example, some platforms may guarantee that callbacks happen on certain threads.

Unsafe wrappers are also useful for prototyping or developing without interference from the borrowchecker,
but with the usual caveat emptor around unsafety and undefined behavior!

# wasm
This crate has full `wasm32-unknown-unknown` support for runtime thread checks around web workers.
*/
pub mod unsafe_send_cell;
pub mod unsafe_sync_cell;
pub mod send_cell;
mod sys;

