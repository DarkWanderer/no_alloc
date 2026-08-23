#[no_alloc_check::no_alloc]
fn safe_sum(buf: &[f32; 3]) -> f32 {
    let [a, b, c] = *buf;
    a + b + c
}

#[no_alloc_check::no_alloc]
fn unsafe_alloc() -> i32 {
    let b = Box::new(5i32);
    *b
}

fn main() {
    let data = [1.0f32, 2.0, 3.0];
    println!("{}", safe_sum(&data));
    println!("{}", unsafe_alloc());
}
