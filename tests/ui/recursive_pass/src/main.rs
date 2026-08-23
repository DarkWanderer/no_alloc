#[no_alloc::no_alloc]
fn root(n: u32) -> u32 {
    if n == 0 {
        0
    } else {
        1 + root(n - 1)
    }
}

fn main() {
    println!("{}", root(10));
}
