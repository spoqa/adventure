//! A trait of responses and common adaptors.
use std::ops::{Deref, DerefMut};
use std::pin::Pin;

use crate::compat::IntoFuture;
use crate::task::{Poll, Waker};

#[cfg(feature = "futures01")]
pub use self::impl_futures01::*;

#[cfg(feature = "std-future")]
pub use self::impl_std::*;

/// Trait to represent types of the response, and the task to receive it.
pub trait Response {
    /// The type of successful values of this response.
    type Ok;
    /// The type of failures of this response.
    type Error;

    /// Poll this [`Response`].
    fn poll(self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>>;

    /// Wrap this response into a type that can work with futures.
    ///
    /// It is compatible with both types of futures 0.1 [`Future`] and
    /// [`std::future::Future`].
    ///
    /// [`Future`]: futures::future::Future
    fn into_future(self) -> IntoFuture<Self>
    where
        Self: Sized,
    {
        IntoFuture::new(self)
    }
}

impl<P> Response for Pin<P>
where
    P: DerefMut + Unpin,
    <P as Deref>::Target: Response,
{
    type Ok = <<P as Deref>::Target as Response>::Ok;
    type Error = <<P as Deref>::Target as Response>::Error;
    fn poll(self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
        let p: Pin<&mut <P as Deref>::Target> = Pin::get_mut(self).as_mut();
        Response::poll(p, w)
    }
}

impl<'a, R: ?Sized> Response for &'a mut R
where
    R: Response + Unpin,
{
    type Ok = R::Ok;
    type Error = R::Error;
    fn poll(mut self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
        let p: Pin<&mut R> = Pin::new(&mut **self);
        Response::poll(p, w)
    }
}

impl<R: ?Sized> Response for Box<R>
where
    R: Response + Unpin,
{
    type Ok = R::Ok;
    type Error = R::Error;
    fn poll(mut self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
        let p: Pin<&mut R> = Pin::new(&mut **self);
        Response::poll(p, w)
    }
}

#[cfg(feature = "futures01")]
mod impl_futures01 {
    use std::pin::Pin;

    use futures::Future;
    use pin_utils::unsafe_pinned;

    use super::Response;
    use crate::task::{Compat, Poll, Waker};

    /// Converts a futures 0.1 [`Future`] into a [`Response`].
    pub struct ResponseFuture<F> {
        inner: Compat<F>,
    }

    impl<F> ResponseFuture<F> {
        unsafe_pinned!(inner: Compat<F>);

        pub fn new(fut: F) -> Self {
            ResponseFuture {
                inner: Compat::new(fut),
            }
        }
    }

    impl<F: Unpin> Unpin for ResponseFuture<F> {}

    impl<F> From<F> for ResponseFuture<F>
    where
        F: Future,
    {
        fn from(fut: F) -> Self {
            ResponseFuture::new(fut)
        }
    }

    impl<F> Response for ResponseFuture<F>
    where
        F: Future,
    {
        type Ok = F::Item;
        type Error = F::Error;

        fn poll(self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            self.inner().poll(w)
        }
    }

    /// A [`Response`] wrapping a trait object of polling futures,
    /// similar to [`Box`]`<dyn `[`Future`]`>`.
    pub struct ResponseLocalFutureObj<'a, T, E> {
        inner: Compat<Box<dyn Future<Item = T, Error = E> + 'a>>,
    }

    impl<'a, T, E> ResponseLocalFutureObj<'a, T, E> {
        unsafe_pinned!(inner: Compat<Box<dyn Future<Item = T, Error = E> + 'a>>);

        pub fn new<F>(fut: F) -> Self
        where
            F: Future<Item = T, Error = E> + 'a,
        {
            ResponseLocalFutureObj {
                inner: Compat::new(Box::new(fut)),
            }
        }
    }

    impl<'a, T, E> Response for ResponseLocalFutureObj<'a, T, E> {
        type Ok = T;
        type Error = E;

        fn poll(self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            self.inner().poll(w)
        }
    }

    /// A [`Response`] wrapping a trait object of polling futures,
    /// similar to [`Box`]`<dyn `[`Future`]` + `[`Send`]` + `[`Sync`]`>`.
    pub struct ResponseFutureObj<'a, T, E> {
        inner: Compat<Box<dyn Future<Item = T, Error = E> + Send + Sync + 'a>>,
    }

    impl<'a, T, E> ResponseFutureObj<'a, T, E> {
        unsafe_pinned!(inner: Compat<Box<dyn Future<Item = T, Error = E> + Send + Sync + 'a>>);

        pub fn new<F>(fut: F) -> Self
        where
            F: Future<Item = T, Error = E> + Send + Sync + 'a,
        {
            ResponseFutureObj {
                inner: Compat::new(Box::new(fut)),
            }
        }
    }

    impl<'a, T, E> Response for ResponseFutureObj<'a, T, E> {
        type Ok = T;
        type Error = E;

        fn poll(self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            self.inner().poll(w)
        }
    }

}

#[cfg(feature = "std-future")]
#[doc(hidden)]
mod impl_std {
    use std::pin::Pin;

    use futures_core::{
        future::{FutureObj, LocalFutureObj},
        Future, TryFuture,
    };
    use pin_utils::unsafe_pinned;

    use super::Response;
    use crate::task::{Poll, Waker};

    /// Converts a [`std::future::Future`] into a [`Response`].
    pub struct ResponseStdFuture<F> {
        inner: F,
    }

    impl<F> ResponseStdFuture<F> {
        unsafe_pinned!(inner: F);

        pub fn new(fut: F) -> Self {
            ResponseStdFuture { inner: fut }
        }
    }

    impl<F: Unpin> Unpin for ResponseStdFuture<F> {}

    impl<F> From<F> for ResponseStdFuture<F>
    where
        F: TryFuture,
    {
        fn from(fut: F) -> Self {
            ResponseStdFuture::new(fut)
        }
    }

    impl<F> Response for ResponseStdFuture<F>
    where
        F: TryFuture,
    {
        type Ok = F::Ok;
        type Error = F::Error;

        fn poll(self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
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

        fn poll(self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
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

        fn poll(self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            TryFuture::try_poll(self.inner(), w)
        }
    }
}
