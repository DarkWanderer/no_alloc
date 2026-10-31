// Both roots are named by one comma-separated `--root` value (see
// `checker-args`). Each must be matched individually: the spec is a list, and
// treating it as a single path reports a spurious "did not match" error.
#[no_alloc_check::no_alloc]
fn a(x: u32) -> u32 {
    x.wrapping_add(1)
}

#[no_alloc_check::no_alloc]
fn b(x: u32) -> u32 {
    x.wrapping_mul(3)
}

fn main() {
    println!("{}", a(1) + b(2));
}
