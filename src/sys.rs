//SPDX-License-Identifier: MIT OR Apache-2.0
#[cfg(target_arch = "wasm32")]
pub use wasm_thread as thread;

#[cfg(not(target_arch = "wasm32"))]
pub use std::thread as thread;