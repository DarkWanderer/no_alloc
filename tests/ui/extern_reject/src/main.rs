unsafe extern "C" {
    fn abs(x: i32) -> i32;
}

#[no_alloc::no_alloc]
fn root() -> i32 {
    unsafe { abs(-5) }
}

fn main() {
    println!("{}", root());
}
