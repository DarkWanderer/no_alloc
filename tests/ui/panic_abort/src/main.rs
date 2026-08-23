#[no_alloc_check::no_alloc]
fn root(values: &[u32], index: usize) -> u32 {
    values[index]
}

fn main() {
    println!("{}", root(&[1], 0));
}
