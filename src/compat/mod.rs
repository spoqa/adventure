#[cfg(feature = "std-futures")]
mod poll {
    pub use std::task::Poll;
}

#[cfg(not(feature = "std-futures"))]
mod poll;

pub use self::poll::Poll;
