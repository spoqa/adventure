use std::result::Result;

/// Indicates whether a value is available or if the current task has been
/// scheduled to receive a wakeup instead.
///
/// ## Note
///
/// This is a copy of [std::task::Poll] to provide the similar interface
/// between using with the 'old-fashioned' [futures 0.1] and the async/await
/// syntax with [std::future].
/// It should be replaced to the standard one after [`futures_api`] has been
/// released on the stable channel.
///
/// Otherwise, you can turn on `std-futures` feature of this crate
/// if you're on the nightly channel.
///
/// [futures 0.1]: https://docs.rs/crate/futures/0.1
/// [`futures_api`]: https://github.com/rust-lang/rust/issues/50547
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Poll<T> {
    /// Represents that a value is immediately ready.
    Ready(T),

    /// Represents that a value is not ready yet.
    ///
    /// When a function returns `Pending`, the function *must* also
    /// ensure that the current task is scheduled to be awoken when
    /// progress can be made.
    Pending,
}

impl<T> Poll<T> {
    /// Change the ready value of this `Poll` with the closure provided
    pub fn map<U, F>(self, f: F) -> Poll<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Poll::Ready(t) => Poll::Ready(f(t)),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Returns whether this is `Poll::Ready`
    #[inline]
    pub fn is_ready(&self) -> bool {
        match *self {
            Poll::Ready(_) => true,
            Poll::Pending => false,
        }
    }

    /// Returns whether this is `Poll::Pending`
    #[inline]
    pub fn is_pending(&self) -> bool {
        !self.is_ready()
    }
}

impl<T, E> Poll<Result<T, E>> {
    /// Change the success value of this `Poll` with the closure provided
    pub fn map_ok<U, F>(self, f: F) -> Poll<Result<U, E>>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Poll::Ready(Ok(t)) => Poll::Ready(Ok(f(t))),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Change the error value of this `Poll` with the closure provided
    pub fn map_err<U, F>(self, f: F) -> Poll<Result<T, U>>
    where
        F: FnOnce(E) -> U,
    {
        match self {
            Poll::Ready(Ok(t)) => Poll::Ready(Ok(t)),
            Poll::Ready(Err(e)) => Poll::Ready(Err(f(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> From<T> for Poll<T> {
    fn from(t: T) -> Poll<T> {
        Poll::Ready(t)
    }
}
