use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

use adventure::{
    oneshot::OneshotRequest,
    paginator::{PagedRequest, Paginator},
    request::{BaseRequest, Request},
};

struct MockClient<T> {
    called: AtomicUsize,
    pred: Box<dyn Fn(&Numbers) -> bool>,
    _phantom: PhantomData<T>,
}

impl<T> MockClient<T> {
    fn new(pred: impl Fn(&Numbers) -> bool + 'static) -> Self {
        MockClient {
            called: Default::default(),
            pred: Box::new(pred),
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug, Default)]
struct Numbers {
    current: AtomicUsize,
    end: usize,
}

macro_rules! test_cases {
    () => {
        impl BaseRequest for &Numbers {
            type Ok = usize;
            type Error = ();
        }

        impl OneshotRequest<&MockClient<Response>> for &Numbers {
            type Response = Response;

            fn send_once(self, client: &MockClient<Response>) -> Self::Response {
                self.send(client)
            }
        }

        impl Request<&MockClient<Response>> for &Numbers {
            type Response = Response;

            fn send(&self, client: &MockClient<Response>) -> Self::Response {
                MockClient::<Response>::send_request(client, self)
            }
        }

        impl PagedRequest<&MockClient<Response>> for &Numbers {
            fn advance(&mut self, response: &Self::Ok) -> bool {
                if *response < self.end {
                    self.current.fetch_add(1, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            }
        }

        #[test]
        fn paginator_basic() {
            let client = MockClient::<Response>::new(|_| true);
            let numbers = Numbers {
                current: AtomicUsize::new(1),
                end: 5,
            };
            let paginator = Paginator::new(&client, &numbers);

            let responses = collect(paginator);
            assert_eq!(Ok(vec![1, 2, 3, 4, 5]), responses);
            assert_eq!(numbers.current.load(Ordering::SeqCst), 5);
            assert_eq!(numbers.end, 5);
            assert_eq!(client.called.load(Ordering::SeqCst), 5);
        }

        #[test]
        fn paginator_basic_2() {
            let client = MockClient::<Response>::new(|n| n.current.load(Ordering::SeqCst) < 7);
            let numbers = Numbers {
                current: AtomicUsize::new(1),
                end: 20,
            };
            let paginator = Paginator::new(&client, &numbers);

            let responses = collect(paginator);
            assert_eq!(Err(()), responses);
            assert_eq!(numbers.current.load(Ordering::SeqCst), 7);
            assert_eq!(client.called.load(Ordering::SeqCst), 7);
        }

        #[test]
        fn paginator_step() {
            let client = MockClient::<Response>::new(|_| true);
            let numbers = Numbers {
                current: AtomicUsize::new(1),
                end: 3,
            };
            let mut paginator = Some(Paginator::new(&client, &numbers));

            assert_eq!(block_on_next(&mut paginator), Some(Ok(1)));
            assert_eq!(numbers.current.load(Ordering::SeqCst), 2);
            assert_eq!(client.called.load(Ordering::SeqCst), 1);

            assert_eq!(block_on_next(&mut paginator), Some(Ok(2)));
            assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
            assert_eq!(client.called.load(Ordering::SeqCst), 2);

            assert_eq!(block_on_next(&mut paginator), Some(Ok(3)));
            assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
            assert_eq!(client.called.load(Ordering::SeqCst), 3);

            assert_eq!(block_on_next(&mut paginator), None);
            assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
            assert_eq!(client.called.load(Ordering::SeqCst), 3);

            assert_eq!(block_on_next(&mut paginator), None);
            assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
            assert_eq!(client.called.load(Ordering::SeqCst), 3);
        }

        #[test]
        fn paginator_step_with_error() {
            let client = MockClient::<Response>::new(|n| n.current.load(Ordering::SeqCst) < 3);
            let numbers = Numbers {
                current: AtomicUsize::new(1),
                end: 3,
            };
            let mut paginator = Some(Paginator::new(&client, &numbers));

            assert_eq!(block_on_next(&mut paginator), Some(Ok(1)));
            assert_eq!(numbers.current.load(Ordering::SeqCst), 2);
            assert_eq!(client.called.load(Ordering::SeqCst), 1);

            assert_eq!(block_on_next(&mut paginator), Some(Ok(2)));
            assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
            assert_eq!(client.called.load(Ordering::SeqCst), 2);

            assert_eq!(block_on_next(&mut paginator), Some(Err(())));
            assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
            assert_eq!(client.called.load(Ordering::SeqCst), 3);

            assert_eq!(block_on_next(&mut paginator), Some(Err(())));
            assert_eq!(numbers.current.load(Ordering::SeqCst), 3);
            assert_eq!(client.called.load(Ordering::SeqCst), 4);
        }
    };
}

#[cfg(all(feature = "futures", not(feature = "futures-util-preview")))]
mod futures01 {
    use super::*;

    use futures::{future, Stream};
    use tokio::runtime::current_thread::block_on_all;

    use adventure::response::ResponseLocalFutureObj;

    pub(super) type Response = ResponseLocalFutureObj<'static, usize, ()>;

    impl MockClient<Response> {
        pub(super) fn send_request(&self, req: &Numbers) -> Response {
            self.called.fetch_add(1, Ordering::SeqCst);
            if (self.pred)(req) {
                Response::new(future::ok(req.current.load(Ordering::SeqCst)))
            } else {
                Response::new(future::err(()))
            }
        }
    }

    pub(super) fn collect<S>(stream: S) -> Result<Vec<S::Item>, S::Error>
    where
        S: Stream,
    {
        block_on_all(stream.collect())
    }

    pub(super) fn block_on_next<S>(stream: &mut Option<S>) -> Option<Result<S::Item, S::Error>>
    where
        S: Stream,
    {
        let (r, s) = match block_on_all(stream.take()?.into_future()) {
            Ok((Some(i), s)) => (Some(Ok(i)), s),
            Ok((None, s)) => (None, s),
            Err((e, s)) => (Some(Err(e)), s),
        };
        stream.replace(s);
        r
    }

    test_cases!();
}

#[cfg(feature = "futures-util-preview")]
mod std_futures {
    use super::*;

    use futures_core::{Stream, TryStream};
    use futures_executor::block_on;
    use futures_util::{future, stream::StreamExt, try_stream::TryStreamExt};

    use adventure::response::ResponseStdLocalFutureObj;

    pub(super) type Response = ResponseStdLocalFutureObj<'static, usize, ()>;

    impl MockClient<Response> {
        pub(super) fn send_request(&self, req: &Numbers) -> Response {
            self.called.fetch_add(1, Ordering::SeqCst);
            if (self.pred)(req) {
                Response::new(future::ok(req.current.load(Ordering::SeqCst)))
            } else {
                Response::new(future::err(()))
            }
        }
    }

    pub(super) fn collect<S>(stream: S) -> Result<Vec<S::Ok>, S::Error>
    where
        S: TryStream,
    {
        block_on(stream.try_collect())
    }

    pub(super) fn block_on_next<S>(stream: &mut Option<S>) -> Option<S::Item>
    where
        S: Stream + Unpin,
    {
        stream.as_mut().and_then(|s| block_on(s.next()))
    }

    test_cases!();
}