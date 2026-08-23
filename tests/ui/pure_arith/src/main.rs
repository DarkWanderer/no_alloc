pub const VALUE: i32 = 1;

pub fn selected(value: u32) -> u32 {
    value
}

#[no_alloc_check::no_alloc]
fn root(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b).wrapping_add(1)
}

fn main() {
    println!("{}", root(3, 4));
}
