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
fn root() -> i32 {
    let obj: &dyn Greet = &En;
    obj.greet()
}

fn main() {
    println!("{}", root());
}
