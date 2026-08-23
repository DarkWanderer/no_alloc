fn is_even(n: u32) -> bool {
    if n == 0 {
        true
    } else {
        is_odd(n - 1)
    }
}

fn is_odd(n: u32) -> bool {
    if n == 0 {
        false
    } else {
        is_even(n - 1)
    }
}

#[no_alloc::no_alloc]
fn root() -> bool {
    is_even(10)
}

fn main() {
    println!("{}", root());
}
