//! A trait of responses and common adaptors.

#[cfg(feature = "futures01")]
pub use self::impl_futures01::*;

#[cfg(feature = "alloc")]
pub use self::impl_std::*;

/// Trait to represent types of the response, and the task to receive it.
pub use futures::future::TryFuture as Response;

#[cfg(feature = "futures01")]
mod impl_futures01 {
    use alloc::boxed::Box;
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    use futures::compat::Compat01As03;
    use futures_01::future::Future as Future01;
    use pin_utils::unsafe_pinned;

    /// Converts a futures 0.1 [`Future01`] into a [`Response`].
    #[must_use = "responses do nothing unless polled"]
    pub struct Future01Response<F> {
        inner: Compat01As03<F>,
    }

    impl<F> Future01Response<F> {
        unsafe_pinned!(inner: Compat01As03<F>);

        pub fn new(fut: F) -> Self {
            Future01Response {
                inner: Compat01As03::new(fut),
            }
        }
    }

    impl<F: Unpin> Unpin for Future01Response<F> {}

    impl<F> From<F> for Future01Response<F>
    where
        F: Future01,
    {
        fn from(fut: F) -> Self {
            Future01Response::new(fut)
        }
    }

    impl<F> Future for Future01Response<F>
    where
        F: Future01,
    {
        type Output = Result<F::Item, F::Error>;

        fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
            self.inner().poll(ctx)
        }
    }

    /// A [`Response`] wrapping a trait object of polling futures,
    /// similar to [`Box`]`<dyn `[`Future01`]`>`.
    #[must_use = "responses do nothing unless polled"]
    pub struct LocalFuture01ResponseObj<'a, T, E> {
        inner: Compat01As03<Box<dyn Future01<Item = T, Error = E> + 'a>>,
    }

    impl<'a, T, E> LocalFuture01ResponseObj<'a, T, E> {
        unsafe_pinned!(inner: Compat01As03<Box<dyn Future01<Item = T, Error = E> + 'a>>);

        pub fn new<F>(fut: F) -> Self
        where
            F: Future01<Item = T, Error = E> + 'a,
        {
            LocalFuture01ResponseObj {
                inner: Compat01As03::new(Box::new(fut)),
            }
        }
    }

    impl<'a, T, E> Future for LocalFuture01ResponseObj<'a, T, E> {
        type Output = Result<T, E>;

        fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
            self.inner().poll(ctx)
        }
    }

    /// A [`Response`] wrapping a trait object of polling futures,
    /// similar to [`Box`]`<dyn `[`Future01`]` + `[`Send`]` + `[`Sync`]`>`.
    #[must_use = "responses do nothing unless polled"]
    pub struct Future01ResponseObj<'a, T, E> {
        inner: Compat01As03<Box<dyn Future01<Item = T, Error = E> + Send + Sync + 'a>>,
    }

    impl<'a, T, E> Future01ResponseObj<'a, T, E> {
        unsafe_pinned!(
            inner: Compat01As03<Box<dyn Future01<Item = T, Error = E> + Send + Sync + 'a>>
        );

        pub fn new<F>(fut: F) -> Self
        where
            F: Future01<Item = T, Error = E> + Send + Sync + 'a,
        {
            Future01ResponseObj {
                inner: Compat01As03::new(Box::new(fut)),
            }
        }
    }

    impl<'a, T, E> Future for Future01ResponseObj<'a, T, E> {
        type Output = Result<T, E>;

        fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
            self.inner().poll(ctx)
        }
    }
}

#[doc(hidden)]
#[cfg(feature = "alloc")]
mod impl_std {
    use alloc::boxed::Box;
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    use futures::future::{FutureObj, LocalFutureObj, TryFuture};
    use pin_utils::unsafe_pinned;

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

    impl<F> Future for FutureResponse<F>
    where
        F: Future,
    {
        type Output = F::Output;

        fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
            Future::poll(self.inner(), ctx)
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

    impl<'a, T, E> Future for LocalFutureResponseObj<'a, T, E> {
        type Output = Result<T, E>;

        fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
            Future::poll(self.inner(), ctx)
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

    impl<'a, T, E> Future for FutureResponseObj<'a, T, E> {
        type Output = Result<T, E>;

        fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
            Future::poll(self.inner(), ctx)
        }
    }
}
