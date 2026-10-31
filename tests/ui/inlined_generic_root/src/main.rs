#[no_alloc_check::no_alloc]
#[inline]
fn make<T: Default>() -> Box<T> {
    Box::new(T::default())
}

fn main() {
    let b = make::<u64>();
    println!("{}", std::hint::black_box(b));
}
