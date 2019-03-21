//! A trait of responses and common adaptors.
use std::pin::Pin;

use crate::compat::Poll;

#[cfg(feature = "futures01")]
pub use self::impl_futures01::*;

#[cfg(feature = "std-futures")]
pub use self::impl_std::*;

/// Trait to represent types of the response, and the task to receive it.
pub trait Response {
    /// The type of successful values of this response.
    type Ok;
    /// The type of failures of this response.
    type Error;
    /// The type of handles for waking up a task by notifying its executor that the response has arrived.
    type Waker;

    /// Poll this [`Response`].
    fn poll(self: Pin<&mut Self>, w: &Self::Waker) -> Poll<Result<Self::Ok, Self::Error>>;
}

#[cfg(feature = "futures01")]
mod impl_futures01 {
    use std::pin::Pin;

    use futures::{Async, Future};

    use super::Response;
    use crate::compat::Poll;

    /// Converts a futures 0.1 [`Future`] into a [`Response`].
    pub struct ResponseFuture<F> {
        inner: F,
    }

    impl<F: Unpin> Unpin for ResponseFuture<F> {}

    impl<F> From<F> for ResponseFuture<F>
    where
        F: Future,
    {
        fn from(fut: F) -> Self {
            ResponseFuture { inner: fut }
        }
    }

    impl<F> Response for ResponseFuture<F>
    where
        F: Future + Unpin,
    {
        type Ok = F::Item;
        type Error = F::Error;
        type Waker = ();

        fn poll(mut self: Pin<&mut Self>, _w: &Self::Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            match Future::poll(&mut self.inner) {
                Ok(Async::Ready(i)) => Poll::Ready(Ok(i)),
                Ok(Async::NotReady) => Poll::Pending,
                Err(e) => Poll::Ready(Err(e)),
            }
        }
    }

    /// A [`Response`] wrapping a trait object of polling futures,
    /// similar to [`Box`]`<dyn `[`Future`]`>`.
    pub struct ResponseLocalFutureObj<'a, T, E> {
        inner: Box<dyn Future<Item = T, Error = E> + 'a>,
    }

    impl<'a, T, E> ResponseLocalFutureObj<'a, T, E> {
        pub fn new<F>(fut: F) -> Self
        where
            F: Future<Item = T, Error = E> + 'a,
        {
            ResponseLocalFutureObj {
                inner: Box::new(fut),
            }
        }

        pub fn into_inner(self) -> Box<dyn Future<Item = T, Error = E> + 'a> {
            self.inner
        }
    }

    impl<'a, T, E> Response for ResponseLocalFutureObj<'a, T, E> {
        type Ok = T;
        type Error = E;
        type Waker = ();

        fn poll(mut self: Pin<&mut Self>, _w: &Self::Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            match Future::poll(&mut self.inner) {
                Ok(Async::Ready(i)) => Poll::Ready(Ok(i)),
                Ok(Async::NotReady) => Poll::Pending,
                Err(e) => Poll::Ready(Err(e)),
            }
        }
    }

    /// A [`Response`] wrapping a trait object of polling futures,
    /// similar to [`Box`]`<dyn `[`Future`]` + `[`Send`]` + `[`Sync`]`>`.
    pub struct ResponseFutureObj<'a, T, E> {
        inner: Box<dyn Future<Item = T, Error = E> + Send + Sync + 'a>,
    }

    impl<'a, T, E> ResponseFutureObj<'a, T, E> {
        pub fn new<F>(fut: F) -> Self
        where
            F: Future<Item = T, Error = E> + Send + Sync + 'a,
        {
            ResponseFutureObj {
                inner: Box::new(fut),
            }
        }

        pub fn into_inner(self) -> Box<dyn Future<Item = T, Error = E> + Send + Sync + 'a> {
            self.inner
        }
    }

    impl<'a, T, E> Response for ResponseFutureObj<'a, T, E> {
        type Ok = T;
        type Error = E;
        type Waker = ();

        fn poll(mut self: Pin<&mut Self>, _w: &Self::Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            match Future::poll(&mut self.inner) {
                Ok(Async::Ready(i)) => Poll::Ready(Ok(i)),
                Ok(Async::NotReady) => Poll::Pending,
                Err(e) => Poll::Ready(Err(e)),
            }
        }
    }

}

#[cfg(feature = "std-futures")]
#[doc(hidden)]
mod impl_std {
    use std::pin::Pin;

    use futures_core::{
        future::{FutureObj, LocalFutureObj},
        task::Waker,
        Future, TryFuture,
    };
    use pin_utils::unsafe_pinned;

    use super::Response;
    use crate::compat::Poll;

    /// Converts a [`std::future::Future`] into a [`Response`].
    pub struct ResponseStdFuture<F> {
        inner: F,
    }

    impl<F> ResponseStdFuture<F> {
        unsafe_pinned!(inner: F);
    }

    impl<F: Unpin> Unpin for ResponseStdFuture<F> {}

    impl<F> From<F> for ResponseStdFuture<F>
    where
        F: TryFuture,
    {
        fn from(fut: F) -> Self {
            ResponseStdFuture { inner: fut }
        }
    }

    impl<F> Response for ResponseStdFuture<F>
    where
        F: TryFuture,
    {
        type Ok = F::Ok;
        type Error = F::Error;
        type Waker = Waker;

        fn poll(self: Pin<&mut Self>, w: &Self::Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            TryFuture::try_poll(self.inner(), w)
        }
    }

    /// A [`Response`] wrapping a trait object of polling futures,
    /// similar to [`LocalFutureObj`].
    pub struct ResponseStdLocalFutureObj<'a, T, E> {
        inner: LocalFutureObj<'a, Result<T, E>>,
    }

    impl<'a, T, E> ResponseStdLocalFutureObj<'a, T, E> {
        unsafe_pinned!(inner: LocalFutureObj<'a, Result<T, E>>);

        pub fn new<F>(fut: F) -> Self
        where
            F: Future<Output = Result<T, E>> + 'a,
        {
            ResponseStdLocalFutureObj {
                inner: LocalFutureObj::new(Box::pin(fut)),
            }
        }

        pub fn into_inner(self) -> LocalFutureObj<'a, Result<T, E>> {
            self.inner
        }
    }

    impl<'a, T, E> Response for ResponseStdLocalFutureObj<'a, T, E> {
        type Ok = T;
        type Error = E;
        type Waker = Waker;

        fn poll(self: Pin<&mut Self>, w: &Self::Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            TryFuture::try_poll(self.inner(), w)
        }
    }

    /// A [`Response`] wrapping a trait object of polling futures,
    /// similar to [`FutureObj`].
    pub struct ResponseStdFutureObj<'a, T, E> {
        inner: FutureObj<'a, Result<T, E>>,
    }

    impl<'a, T, E> ResponseStdFutureObj<'a, T, E> {
        unsafe_pinned!(inner: FutureObj<'a, Result<T, E>>);

        pub fn new<F>(fut: F) -> Self
        where
            F: Future<Output = Result<T, E>> + Send + 'a,
        {
            ResponseStdFutureObj {
                inner: FutureObj::new(Box::pin(fut)),
            }
        }

        pub fn into_inner(self) -> FutureObj<'a, Result<T, E>> {
            self.inner
        }
    }

    impl<'a, T, E> Response for ResponseStdFutureObj<'a, T, E> {
        type Ok = T;
        type Error = E;
        type Waker = Waker;

        fn poll(self: Pin<&mut Self>, w: &Self::Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            TryFuture::try_poll(self.inner(), w)
        }
    }
}
