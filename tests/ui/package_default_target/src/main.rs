#[no_alloc_check::no_alloc]
#[inline(never)]
fn root(x: u8) -> u8 {
    x.wrapping_add(1)
}

fn main() {
    println!("{}", root(1));
}
