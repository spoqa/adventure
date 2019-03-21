#[cfg(feature = "std-futures")]
pub use std::task::Poll;

#[cfg(not(feature = "std-futures"))]
pub use self::internal::*;

#[cfg(not(feature = "std-futures"))]
#[doc(hidden)]
mod internal {
    pub enum Poll<T> {
        Ready(T),
        Pending,
    }
}
