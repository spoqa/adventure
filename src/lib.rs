//! A general method of the common pattern for network requests.
//!
//! This crate defines a general interface of the [request-response pattern]
//! like HTTP request, and provides a number of composable constructs to work
//! with it at the high-level.
//!
//! [request-response pattern]: https://en.wikipedia.org/wiki/Request%E2%80%93response
#![cfg_attr(feature = "std-future", feature(futures_api))]
#![deny(rust_2018_idioms)]

pub mod compat;
pub mod prelude;
pub mod request;
pub mod response;

mod adaptor;
mod paginator;

#[cfg(test)]
mod test;

#[doc(inline)]
pub use crate::{
    paginator::Paginator,
    request::{PagedRequest, Request},
    response::Response,
};
