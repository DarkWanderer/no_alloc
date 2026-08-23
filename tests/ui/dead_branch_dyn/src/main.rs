trait Greet {
    fn greet(&self) -> i32;
}
struct En;
impl Greet for En {
    fn greet(&self) -> i32 {
        1
    }
}

#[no_alloc::no_alloc]
fn root(take_dyn_branch: bool) -> i32 {
    if take_dyn_branch {
        let obj: &dyn Greet = &En;
        return obj.greet();
    }
    42
}

fn main() {
    println!("{}", root(false));
}
