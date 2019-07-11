//! A general method of the common pattern for network requests.
//!
//! This crate defines a general interface of the [request-response pattern]
//! like HTTP request, and provides a number of composable constructs to work
//! with it at the high-level.
//!
//! [request-response pattern]: https://en.wikipedia.org/wiki/Request%E2%80%93response
//!
//! # Examples
//!
//! ```
//! # #[cfg(feature = "reqwest")]
//! # { (|| {
//! use std::time::Duration;
//!
//! use adventure::prelude::*;
//! use adventure::response::LocalFuture01ResponseObj;
//! use futures::Future;
//! use reqwest::{r#async::Client, Error};
//! use serde::Deserialize;
//!
//! // declare the request and the desired output.
//! struct GetRepo<'a> {
//!     owner: &'a str,
//!     repo: &'a str,
//! };
//!
//! #[derive(Deserialize)]
//! struct Repository {
//!     id: u64,
//!     name: String,
//!     full_name: String,
//!     description: String,
//! }
//!
//! // describe the relation of request, result, and error.
//! impl BaseRequest for GetRepo<'_> {
//!     type Ok = Repository;
//!     type Error = Error;
//! }
//!
//! impl<'a> Request<&'a Client> for GetRepo<'_> {
//!     // convenient wrapper for boxed futures
//!     type Response = LocalFuture01ResponseObj<'a, Self::Ok, Self::Error>;
//!
//!     // implement how to send the request and extract the result.
//!     fn send(mut self: Pin<&mut Self>, client: &'a Client) -> Self::Response {
//!         let url = format!("https://api.github.com/repos/{}/{}", self.owner, self.repo);
//!         let resp = client.get(&url)
//!             .header("Accept", "application/vnd.github.v3+json")
//!             .send()
//!             .and_then(|mut r| r.json());
//!         LocalFuture01ResponseObj::new(resp)
//!     }
//! }
//!
//! // this is for `retry()` method.
//! impl RetriableRequest for GetRepo<'_> {
//!     fn should_retry(&self, err: &Self::Error, next_duration: Duration) -> bool {
//!         err.is_server_error()
//!     }
//! }
//!
//! // let's try it
//!
//! use tokio::runtime::current_thread::block_on_all;
//!
//! let client = Client::new();
//! let request = GetRepo { owner: "spoqa", repo: "adventure" };
//! let response = request
//!     .retry()
//!     .send_once(&client);
//! let repo = block_on_all(response.into_future())?;
//! assert_eq!(repo.description, "Helps your great adventure for the various type of requests.");
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! # })().unwrap(); }
//! ```
#![deny(rust_2018_idioms)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod oneshot;
pub mod paginator;
pub mod prelude;
pub mod repeat;
pub mod request;
pub mod response;

#[cfg(feature = "backoff")]
pub mod retry;

#[doc(inline)]
pub use crate::{
    oneshot::OneshotRequest,
    paginator::{PagedRequest, Paginator},
    request::{BaseRequest, Request},
    response::Response,
};

#[cfg(feature = "backoff")]
#[doc(inline)]
pub use crate::retry::RetriableRequest;
