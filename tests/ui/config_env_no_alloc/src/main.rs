#[no_alloc_check::no_alloc]
#[inline(never)]
fn root(x: u8) -> u8 {
    *Box::new(x)
}

fn main() {
    println!("{}", root(1));
}
