use std::time::Duration;

use crate::repeat::Repeat;
use crate::request::BaseRequest;
use crate::response::Response;
#[cfg(feature = "tokio-timer")]
use crate::retry::TokioTimer;
use crate::retry::{Backoff, RetriableRequest, Retrying, Timer};

pub trait RequestExt {
    type Ok;
    type Error;

    fn repeat(self) -> Repeat<Self>
    where
        Self: Clone,
    {
        Repeat::from(self)
    }

    #[cfg(feature = "tokio-timer")]
    fn with_backoff(self) -> Retrying<Self, TokioTimer>
    where
        Self: RetriableRequest + Sized,
    {
        Retrying::new(self)
    }

    #[cfg(feature = "tokio-timer")]
    fn with_backoff_if<F>(self, pred: F) -> Retrying<Self, TokioTimer, F>
    where
        Self: BaseRequest + Sized,
        F: Fn(&Self, &<Self as BaseRequest>::Error, Duration) -> bool,
    {
        Retrying::with_predicate(self, pred)
    }

    fn with_backoff_config<T, F, B>(self, timer: T, pred: F, backoff: B) -> Retrying<Self, T, F, B>
    where
        Self: BaseRequest + Sized,
        T: Timer + Unpin,
        F: Fn(&Self, &<Self as BaseRequest>::Error, Duration) -> bool,
        B: Backoff,
    {
        Retrying::with_config(self, timer, pred, backoff)
    }
}

impl<T> RequestExt for T
where
    T: BaseRequest,
{
    type Ok = T::Ok;
    type Error = T::Error;
}

pub trait ResponseExt {
    fn into_future(self) -> IntoFuture<Self>
    where
        Self: Sized;
}

impl<T> ResponseExt for T
where
    T: Response,
{
    fn into_future(self) -> IntoFuture<Self>
    where
        Self: Sized,
    {
        IntoFuture(self)
    }
}

pub struct IntoFuture<T>(T);

#[cfg(feature = "futures01")]
mod impl_futures01 {
    use futures::{Future as Future01, Poll as Poll01};

    use crate::response::Response;
    use crate::task::convert_std_to_01;

    use super::IntoFuture;

    impl<T> Future01 for IntoFuture<T>
    where
        T: Response + Unpin,
    {
        type Item = T::Ok;
        type Error = T::Error;

        fn poll(&mut self) -> Poll01<Self::Item, Self::Error> {
            internal::with_context(self, |inner, w| convert_std_to_01(Response::poll(inner, w)))
        }
    }

    #[cfg(feature = "std-future")]
    mod internal {
        // Copied from futures 0.3.0-alpha.1
        // Should be replaced if `futures-api` has been stablized
        use std::mem;
        use std::pin::Pin;
        use std::sync::Arc;
        use std::task::{RawWaker, RawWakerVTable};

        use futures::task as task01;
        use futures_util::task::{ArcWake, WakerRef};

        use super::IntoFuture;
        use crate::task::Waker;

        #[derive(Clone)]
        struct Current(task01::Task);

        impl Current {
            fn new() -> Current {
                Current(task01::current())
            }

            fn as_waker(&self) -> WakerRef<'_> {
                unsafe fn ptr_to_current<'a>(ptr: *const ()) -> &'a Current {
                    &*(ptr as *const Current)
                }
                fn current_to_ptr(current: &Current) -> *const () {
                    current as *const Current as *const ()
                }

                unsafe fn clone(ptr: *const ()) -> RawWaker {
                    // Lazily create the `Arc` only when the waker is actually cloned.
                    // FIXME: remove `transmute` when a `Waker` -> `RawWaker` conversion
                    // function is landed in `core`.
                    mem::transmute::<Waker, RawWaker>(
                        Arc::new(ptr_to_current(ptr).clone()).into_waker(),
                    )
                }
                unsafe fn drop(_: *const ()) {}
                unsafe fn wake(ptr: *const ()) {
                    ptr_to_current(ptr).0.notify()
                }

                let ptr = current_to_ptr(self);
                let vtable = &RawWakerVTable { clone, drop, wake };
                unsafe { WakerRef::new(Waker::new_unchecked(RawWaker::new(ptr, vtable))) }
            }
        }

        impl ArcWake for Current {
            fn wake(arc_self: &Arc<Self>) {
                arc_self.0.notify();
            }
        }

        pub(super) fn with_context<T, R, F>(fut: &mut IntoFuture<T>, f: F) -> R
        where
            T: Unpin,
            F: FnOnce(Pin<&mut T>, &Waker) -> R,
        {
            let current = Current::new();
            let waker = current.as_waker();
            f(Pin::new(&mut fut.0), &waker)
        }
    }

    #[cfg(not(feature = "std-future"))]
    mod internal {
        use std::pin::Pin;

        use crate::task::Waker;

        use super::*;

        pub(super) fn with_context<T, R, F>(fut: &mut IntoFuture<T>, f: F) -> R
        where
            T: Unpin,
            F: FnOnce(Pin<&mut T>, &Waker) -> R,
        {
            let waker = unsafe { Waker::blank() };
            f(Pin::new(&mut fut.0), &waker)
        }
    }
}

#[cfg(feature = "std-future")]
mod impl_std {
    use std::pin::Pin;

    use futures_core::Future;

    use crate::response::Response;
    use crate::task::{Poll, Waker};

    use super::IntoFuture;

    impl<T> Future for IntoFuture<T>
    where
        T: Response + Unpin,
    {
        type Output = Result<T::Ok, T::Error>;

        fn poll(mut self: Pin<&mut Self>, w: &Waker) -> Poll<Self::Output> {
            Response::poll(Pin::new(&mut self.0), w)
        }
    }
}
