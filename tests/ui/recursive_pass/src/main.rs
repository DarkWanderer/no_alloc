#[no_alloc_check::no_alloc]
fn root(n: u32) -> u32 {
    if n == 0 {
        0
    } else {
        root(n.wrapping_sub(1)).wrapping_add(1)
    }
}

fn main() {
    println!("{}", root(10));
}
