fn is_even(n: u32) -> bool {
    if n == 0 {
        true
    } else {
        is_odd(n.wrapping_sub(1))
    }
}

fn is_odd(n: u32) -> bool {
    if n == 0 {
        false
    } else {
        is_even(n.wrapping_sub(1))
    }
}

#[no_alloc_check::no_alloc]
fn root() -> bool {
    is_even(10)
}

fn main() {
    println!("{}", root());
}
