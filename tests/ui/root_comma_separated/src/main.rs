// Both roots are named by one comma-separated `--root` value (see
// `checker-args`). Each must be matched individually: the spec is a list, and
// treating it as a single path reports a spurious "did not match" error.
// Only `a` carries a marker, so `b` is checked solely because `--root`
// selected it; a partial split of the list would drop `b` from the report.
#[no_alloc_check::no_alloc]
fn a(x: u32) -> u32 {
    x.wrapping_add(1)
}

fn b(x: u32) -> u32 {
    x.wrapping_mul(3)
}

fn main() {
    println!("{}", a(1) + b(2));
}
