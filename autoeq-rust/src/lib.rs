pub mod api;
pub mod constants;
pub mod csv;
pub mod dsp;
pub mod error;
pub mod ffi;
pub mod frequency_response;
pub mod peq;
/// HTTP 服务端（axum/tokio）只在非 wasm 平台编译，wasm32 上没有网络栈
#[cfg(not(target_arch = "wasm32"))]
pub mod server;
pub mod utils;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub use error::{AutoEqError, Result};
