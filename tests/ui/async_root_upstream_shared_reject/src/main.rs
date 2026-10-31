use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

fn main() {
    let mut f = pin!(shared_dep::work::<u64>());
    let mut cx = Context::from_waker(Waker::noop());
    if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
        println!("{v}");
    }
}
