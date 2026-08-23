fn helper() -> i32 {
    7
}

#[no_alloc_check::no_alloc]
fn root() -> i32 {
    let f: fn() -> i32 = helper;
    f()
}

fn main() {
    println!("{}", root());
}
