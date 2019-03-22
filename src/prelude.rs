//! A prelude of the `adventure` for the crate which want to try with it.
//!
//! This module is intended to be included by `use adventure::prelude::*;`,
//! to access the various traits and methods mostly will be used.

pub use crate::adaptor::ResponseExt;
pub use crate::request::{PagedRequest, Request};
pub use crate::response::Response;
