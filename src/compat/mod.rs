//! A types for compatibility with futures 0.1 crate.

#[cfg(feature = "std-futures")]
mod poll {
    pub use std::task::Poll;
}

#[cfg(not(feature = "std-futures"))]
mod poll;

pub use self::poll::Poll;
