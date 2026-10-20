//! AIP-193: Error handling

#[cfg(feature = "derive")]
pub use aip_193_derive::IntoStatus;

#[doc(hidden)]
pub mod __private {
    pub use super::Code;
    pub use super::status::Status;
    pub use super::traits::IntoStatus;
    pub use std::collections::HashMap;
}

#[cfg(feature = "axum")]
mod axum_impl;
mod code;
#[cfg(feature = "http")]
mod http_impl;

mod error_details;
#[cfg(feature = "proto")]
pub mod proto;
mod status;
mod traits;

pub use code::Code;
pub use error_details::{ErrorInfo, ErrorInfoBuilder};
pub use status::{Status, StatusDetails};
pub use traits::IntoStatus;
