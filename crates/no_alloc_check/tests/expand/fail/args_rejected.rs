#[no_alloc_check::no_alloc(oops)]
fn compute(x: i32) -> i32 {
    x + 1
}

fn main() {}
