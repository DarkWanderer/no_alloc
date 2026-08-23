trait Greet {
    #[allow(dead_code)]
    fn greet(&self) -> i32;
}
struct En;
impl Greet for En {
    fn greet(&self) -> i32 {
        1
    }
}

#[no_alloc_check::no_alloc]
fn root(value: Box<dyn Greet>) {
    let _ = value;
}

fn main() {
    root(Box::new(En));
}
