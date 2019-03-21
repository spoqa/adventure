//! A types for compatibility with futures 0.1 crate.

#[cfg(feature = "std-futures")]
mod internal {
    pub use std::task::{Poll, Waker};
}

#[cfg(not(feature = "std-futures"))]
mod internal;

#[doc(inline)]
pub use self::internal::*;

#[cfg(feature = "futures01")]
pub(crate) use self::internal_futures01::*;

#[cfg(feature = "futures01")]
mod internal_futures01 {
    use std::pin::Pin;

    use futures::{Async, Future as Future01, Poll as Poll01};
    use pin_utils::unsafe_unpinned;

    use crate::response::Response;

    use super::*;

    pub(crate) fn convert_01_to_std<T, E>(poll: Poll01<T, E>) -> Poll<Result<T, E>> {
        match poll {
            Ok(Async::Ready(i)) => Poll::Ready(Ok(i)),
            Ok(Async::NotReady) => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    #[cfg(feature = "std-futures")]
    type Wrap<T> = crate::response::ResponseStdFuture<futures_util::compat::Compat01As03<T>>;

    #[cfg(not(feature = "std-futures"))]
    type Wrap<T> = T;

    pub struct Compat<T> {
        inner: Wrap<T>,
    }

    impl<T> Compat<T> {
        unsafe_unpinned!(inner: Wrap<T>);

        #[cfg(feature = "std-futures")]
        pub(crate) fn new(object: T) -> Self {
            let object = futures_util::compat::Compat01As03::new(object);
            Compat {
                inner: crate::response::ResponseStdFuture::new(object),
            }
        }

        #[cfg(not(feature = "std-futures"))]
        pub(crate) fn new(object: T) -> Self {
            Compat { inner: object }
        }
    }

    impl<T> Response for Compat<T>
    where
        T: Future01,
    {
        type Ok = T::Item;
        type Error = T::Error;

        #[cfg(feature = "std-futures")]
        fn poll(mut self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            Pin::new(&mut self.inner).poll(w)
        }

        #[cfg(not(feature = "std-futures"))]
        fn poll(self: Pin<&mut Self>, _w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            convert_01_to_std(Future01::poll(self.inner()))
        }
    }

}
