use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

pub async fn work() -> usize {
    let b = Box::new(7usize);
    *b
}

fn main() {
    let mut f = pin!(work());
    let mut cx = Context::from_waker(Waker::noop());
    if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { println!("{v}"); }
}
