pub async fn work<T: Default>() -> T {
    let b = Box::new(T::default());
    *b
}
