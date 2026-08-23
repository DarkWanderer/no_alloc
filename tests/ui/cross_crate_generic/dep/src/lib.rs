pub trait Work {
    fn work() -> usize;
}

#[no_alloc_check::no_alloc]
pub fn root<T: Work>() -> usize {
    T::work()
}
