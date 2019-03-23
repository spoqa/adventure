//! A prelude of the `adventure` for the crate which want to try with it.
//!
//! This module is intended to be included by `use adventure::prelude::*;`,
//! to access the various traits and methods mostly will be used.

pub use crate::paginator::PagedRequest;
pub use crate::repeat::RepeatableRequest;
pub use crate::request::Request;
pub use crate::response::Response;
pub use crate::retry::RetriableRequest;
pub use crate::util::{RequestExt, ResponseExt};
