use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[cfg(all(feature = "futures", not(feature = "std-future")))]
use futures::future;
#[cfg(feature = "std-future")]
use futures_util::future;
use pin_utils::pin_mut;
use tokio::runtime::current_thread::block_on_all;

use adventure::prelude::*;
use adventure::response::*;

#[derive(Debug, Default)]
pub(crate) struct Numbers {
    current: AtomicUsize,
    end: usize,
}

impl Clone for Numbers {
    fn clone(&self) -> Self {
        Numbers {
            current: AtomicUsize::new(self.current.load(Ordering::SeqCst)),
            end: self.end,
        }
    }
}

#[cfg(feature = "std-future")]
type Resp = ResponseStdFutureObj<'static, usize, String>;
#[cfg(all(feature = "futures", not(feature = "std-future")))]
type Resp = ResponseFutureObj<'static, usize, String>;

impl BaseRequest for Numbers {
    type Ok = usize;
    type Error = String;
}

impl<C> Request<C> for Numbers {
    type Response = Resp;

    fn into_response(self, client: C) -> Self::Response {
        self.send(client)
    }
}

impl<C> RepeatableRequest<C> for Numbers {
    type Response = Resp;

    fn send(&self, _client: C) -> Self::Response {
        let i = self.current.fetch_add(1, Ordering::SeqCst);
        if i < self.end {
            Resp::new(future::err(format!("{} tried", i)))
        } else {
            Resp::new(future::ok(i))
        }
    }
}

impl RetriableRequest for Numbers {
    fn should_retry(&self, _error: &Self::Error, _next_interval: Duration) -> bool {
        true
    }
}

fn block_on<R>(req: R) -> Result<R::Ok, R::Error>
where
    R: Response + Unpin,
{
    let fut = req.into_future();
    #[cfg(all(feature = "std-future", not(feature = "futures")))]
    let fut = fut.compat();
    block_on_all(fut)
}

#[test]
fn retry_simple() {
    let numbers = Numbers {
        current: AtomicUsize::new(1),
        end: 5,
    };
    pin_mut!(numbers);
    let res = numbers.with_backoff().into_response(());

    assert_eq!(block_on(res).unwrap(), 5);
}
