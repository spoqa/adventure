//! A types for compatibility with futures 0.1 crate.

mod internal;

pub use core::task::{Context, Poll, Waker};

pub use self::internal::*;

#[cfg(feature = "futures01")]
pub use futures_util::compat::Compat01As03 as Compat;
