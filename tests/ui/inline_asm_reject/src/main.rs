use std::arch::asm;

#[no_alloc::no_alloc]
fn root() -> i32 {
    let x: i32;
    unsafe {
        asm!("mov {0}, 5", out(reg) x);
    }
    x
}

fn main() {
    println!("{}", root());
}
