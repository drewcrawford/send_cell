//SPDX-License-Identifier: MIT OR Apache-2.0
/*!
Platform abstraction for threading primitives.

This module provides a unified interface for accessing thread-related functionality
across different platforms. The main purpose is to abstract over the differences
between standard native threading (using `std::thread`) and WebAssembly threading
(using the `wasm_thread` crate).

# Platform Support

## Native Platforms

On native platforms (non-WebAssembly), this module re-exports `std::thread`,
providing access to the standard library's threading primitives.

## WebAssembly

On WebAssembly (`wasm32-unknown-unknown`), this module re-exports the `wasm_thread`
crate, which provides Web Worker-based threading support. This allows the send_cells
crate to work correctly in browser environments with Web Workers.

# Usage

This module is primarily used internally by the send_cells crate to:
- Get current thread IDs for runtime checking in [`crate::SendCell`]
- Provide thread-safe abstractions that work across platforms
- Enable consistent behavior between native and WebAssembly environments

# Examples

```rust
// Get the current thread ID (works on both native and WASM)
let thread_id = send_cells::sys::thread::current().id();
println!("Current thread: {:?}", thread_id);
```

# Dependencies

On WebAssembly platforms, this module depends on the `wasm_thread` crate,
which is automatically included when building for `wasm32-unknown-unknown`.
On other platforms, it uses the standard library's `std::thread`.
*/

#[cfg(target_arch = "wasm32")]
pub use wasm_thread as thread;

#[cfg(not(target_arch = "wasm32"))]
pub use std::thread as thread;