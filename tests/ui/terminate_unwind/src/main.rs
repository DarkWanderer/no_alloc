#[inline(never)]
fn leaf(value: u32) -> u32 {
    value
}

#[no_alloc_check::no_alloc]
extern "C" fn root(value: u32) -> u32 {
    leaf(value)
}

fn main() {
    println!("{}", root(1));
}
