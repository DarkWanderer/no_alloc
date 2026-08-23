#[no_alloc_check::no_alloc]
fn call_closure<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 {
    f(x)
}

#[no_alloc_check::no_alloc]
fn sum_via_for_loop(buf: &[u32]) -> u32 {
    let mut total = 0u32;
    for &x in buf {
        total = total.wrapping_add(x);
    }
    total
}

fn main() {
    println!("{}", call_closure(|x| x.wrapping_add(1), 41));
    println!("{}", sum_via_for_loop(&[1, 2, 3]));
}
