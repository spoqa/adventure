#![cfg_attr(feature = "std-futures", feature(futures_api))]
#![deny(rust_2018_idioms)]

pub mod compat;
pub mod paginator;
pub mod prelude;
pub mod request;
pub mod response;

#[cfg(test)]
mod test;

pub use crate::paginator::Paginator;
pub use crate::request::{PagedRequest, Request};
pub use crate::response::Response;
