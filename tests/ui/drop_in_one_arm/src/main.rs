#[no_alloc_check::no_alloc]
fn root(take_branch: bool) -> i32 {
    if take_branch {
        let mut v: Vec<i32> = Vec::new();
        v.push(1);
        drop(v);
    }
    42
}

fn main() {
    println!("{}", root(false));
}
