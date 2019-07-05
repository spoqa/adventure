use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures_util::future;
use futures_util::TryFutureExt;
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

type Resp = FutureResponseObj<'static, usize, String>;

impl BaseRequest for Numbers {
    type Ok = usize;
    type Error = String;
}

impl<C> OneshotRequest<C> for Numbers {
    type Response = Resp;

    fn send_once(mut self, client: C) -> Self::Response {
        Pin::new(&mut self).send(client)
    }
}

impl<C> Request<C> for Numbers {
    type Response = Resp;

    fn send(self: Pin<&mut Self>, _client: C) -> Self::Response {
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
    let fut = req.compat();
    block_on_all(fut)
}

#[test]
fn retry_send_once() {
    let numbers = Numbers {
        current: AtomicUsize::new(1),
        end: 5,
    };
    pin_mut!(numbers);
    let res = numbers.retry().send_once(());

    assert_eq!(block_on(res).unwrap(), 5);
}

#[test]
fn retry_clone() {
    let numbers = Numbers {
        current: AtomicUsize::new(1),
        end: 5,
    };
    let cloned = numbers.retry().clone();

    assert_eq!(block_on(cloned.send_once(())).unwrap(), 5);
}

#[test]
fn retry_send() {
    let numbers = Numbers {
        current: AtomicUsize::new(1),
        end: 5,
    };
    let req = numbers.retry();
    pin_mut!(req);
    let res = req.send(());

    assert_eq!(block_on(res).unwrap(), 5);
}
