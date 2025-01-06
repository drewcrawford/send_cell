/*!
Cell types that can be sent/shared between threads, even if the underlying data cannot.

*/
pub mod unsafe_send_cell;
pub mod unsafe_sync_cell;
pub mod send_cell;
mod sys;

