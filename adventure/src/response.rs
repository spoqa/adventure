//! A trait of responses and common adaptors.
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::Context;

use crate::compat::IntoFuture;
use crate::task::Poll;

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
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<Self::Ok, Self::Error>>;

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
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<Self::Ok, Self::Error>> {
        let p: Pin<&mut <P as Deref>::Target> = Pin::get_mut(self).as_mut();
        Response::poll(p, ctx)
    }
}

impl<'a, R: ?Sized> Response for &'a mut R
where
    R: Response + Unpin,
{
    type Ok = R::Ok;
    type Error = R::Error;
    fn poll(
        mut self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<Self::Ok, Self::Error>> {
        let p: Pin<&mut R> = Pin::new(&mut **self);
        Response::poll(p, ctx)
    }
}

impl<R: ?Sized> Response for Box<R>
where
    R: Response + Unpin,
{
    type Ok = R::Ok;
    type Error = R::Error;
    fn poll(
        mut self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<Self::Ok, Self::Error>> {
        let p: Pin<&mut R> = Pin::new(&mut **self);
        Response::poll(p, ctx)
    }
}

#[cfg(feature = "futures01")]
mod impl_futures01 {
    use std::pin::Pin;
    use std::task::Context;

    use futures::Future;
    use pin_utils::unsafe_pinned;

    use super::Response;
    use crate::task::{Compat, Poll};

    /// Converts a futures 0.1 [`Future`] into a [`Response`].
    #[must_use = "responses do nothing unless polled"]
    pub struct Future01Response<F> {
        inner: Compat<F>,
    }

    impl<F> Future01Response<F> {
        unsafe_pinned!(inner: Compat<F>);

        pub fn new(fut: F) -> Self {
            Future01Response {
                inner: Compat::new(fut),
            }
        }
    }

    impl<F: Unpin> Unpin for Future01Response<F> {}

    impl<F> From<F> for Future01Response<F>
    where
        F: Future,
    {
        fn from(fut: F) -> Self {
            Future01Response::new(fut)
        }
    }

    impl<F> Response for Future01Response<F>
    where
        F: Future,
    {
        type Ok = F::Item;
        type Error = F::Error;

        fn poll(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
        ) -> Poll<Result<Self::Ok, Self::Error>> {
            self.inner().poll(ctx)
        }
    }

    /// A [`Response`] wrapping a trait object of polling futures,
    /// similar to [`Box`]`<dyn `[`Future`]`>`.
    #[must_use = "responses do nothing unless polled"]
    pub struct LocalFuture01ResponseObj<'a, T, E> {
        inner: Compat<Box<dyn Future<Item = T, Error = E> + 'a>>,
    }

    impl<'a, T, E> LocalFuture01ResponseObj<'a, T, E> {
        unsafe_pinned!(inner: Compat<Box<dyn Future<Item = T, Error = E> + 'a>>);

        pub fn new<F>(fut: F) -> Self
        where
            F: Future<Item = T, Error = E> + 'a,
        {
            LocalFuture01ResponseObj {
                inner: Compat::new(Box::new(fut)),
            }
        }
    }

    impl<'a, T, E> Response for LocalFuture01ResponseObj<'a, T, E> {
        type Ok = T;
        type Error = E;

        fn poll(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
        ) -> Poll<Result<Self::Ok, Self::Error>> {
            self.inner().poll(ctx)
        }
    }

    /// A [`Response`] wrapping a trait object of polling futures,
    /// similar to [`Box`]`<dyn `[`Future`]` + `[`Send`]` + `[`Sync`]`>`.
    #[must_use = "responses do nothing unless polled"]
    pub struct Future01ResponseObj<'a, T, E> {
        inner: Compat<Box<dyn Future<Item = T, Error = E> + Send + Sync + 'a>>,
    }

    impl<'a, T, E> Future01ResponseObj<'a, T, E> {
        unsafe_pinned!(inner: Compat<Box<dyn Future<Item = T, Error = E> + Send + Sync + 'a>>);

        pub fn new<F>(fut: F) -> Self
        where
            F: Future<Item = T, Error = E> + Send + Sync + 'a,
        {
            Future01ResponseObj {
                inner: Compat::new(Box::new(fut)),
            }
        }
    }

    impl<'a, T, E> Response for Future01ResponseObj<'a, T, E> {
        type Ok = T;
        type Error = E;

        fn poll(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
        ) -> Poll<Result<Self::Ok, Self::Error>> {
            self.inner().poll(ctx)
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
    use crate::task::{Context, Poll};

    /// Converts a [`std::future::Future`] into a [`Response`].
    #[must_use = "responses do nothing unless polled"]
    pub struct FutureResponse<F> {
        inner: F,
    }

    impl<F> FutureResponse<F> {
        unsafe_pinned!(inner: F);

        pub fn new(fut: F) -> Self {
            FutureResponse { inner: fut }
        }
    }

    impl<F: Unpin> Unpin for FutureResponse<F> {}

    impl<F> From<F> for FutureResponse<F>
    where
        F: TryFuture,
    {
        fn from(fut: F) -> Self {
            FutureResponse::new(fut)
        }
    }

    impl<F> Response for FutureResponse<F>
    where
        F: TryFuture,
    {
        type Ok = F::Ok;
        type Error = F::Error;

        fn poll(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
        ) -> Poll<Result<Self::Ok, Self::Error>> {
            TryFuture::try_poll(self.inner(), ctx)
        }
    }

    /// A [`Response`] wrapping a trait object of polling futures,
    /// similar to [`LocalFutureObj`].
    #[must_use = "responses do nothing unless polled"]
    pub struct LocalFutureResponseObj<'a, T, E> {
        inner: LocalFutureObj<'a, Result<T, E>>,
    }

    impl<'a, T, E> LocalFutureResponseObj<'a, T, E> {
        unsafe_pinned!(inner: LocalFutureObj<'a, Result<T, E>>);

        pub fn new<F>(fut: F) -> Self
        where
            F: Future<Output = Result<T, E>> + 'a,
        {
            LocalFutureResponseObj {
                inner: LocalFutureObj::new(Box::pin(fut)),
            }
        }

        pub fn into_inner(self) -> LocalFutureObj<'a, Result<T, E>> {
            self.inner
        }
    }

    impl<'a, T, E> Response for LocalFutureResponseObj<'a, T, E> {
        type Ok = T;
        type Error = E;

        fn poll(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
        ) -> Poll<Result<Self::Ok, Self::Error>> {
            TryFuture::try_poll(self.inner(), ctx)
        }
    }

    /// A [`Response`] wrapping a trait object of polling futures,
    /// similar to [`FutureObj`].
    #[must_use = "responses do nothing unless polled"]
    pub struct FutureResponseObj<'a, T, E> {
        inner: FutureObj<'a, Result<T, E>>,
    }

    impl<'a, T, E> FutureResponseObj<'a, T, E> {
        unsafe_pinned!(inner: FutureObj<'a, Result<T, E>>);

        pub fn new<F>(fut: F) -> Self
        where
            F: Future<Output = Result<T, E>> + Send + 'a,
        {
            FutureResponseObj {
                inner: FutureObj::new(Box::pin(fut)),
            }
        }

        pub fn into_inner(self) -> FutureObj<'a, Result<T, E>> {
            self.inner
        }
    }

    impl<'a, T, E> Response for FutureResponseObj<'a, T, E> {
        type Ok = T;
        type Error = E;

        fn poll(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
        ) -> Poll<Result<Self::Ok, Self::Error>> {
            TryFuture::try_poll(self.inner(), ctx)
        }
    }
}
