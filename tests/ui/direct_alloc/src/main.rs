#[no_alloc_check::no_alloc]
fn root() -> i32 {
    let b = Box::new(5i32);
    *b
}

fn main() {
    println!("{}", root());
}
