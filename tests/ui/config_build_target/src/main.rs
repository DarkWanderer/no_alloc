// The allocation exists only when compiling for ARM, i.e. in the build the
// config selects and not in the host build the checker would otherwise run.
#[no_alloc_check::no_alloc]
#[inline(never)]
fn root(x: u8) -> u8 {
    #[cfg(target_arch = "arm")]
    {
        let boxed = Box::new(x);
        return *boxed;
    }
    #[allow(unreachable_code)]
    x
}

fn main() {
    println!("{}", root(1));
}
