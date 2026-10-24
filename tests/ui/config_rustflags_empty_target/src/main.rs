// Same as `config_rustflags`, plus empty `target.<host>.rustflags` tables that
// must not hide `build.rustflags`.
// The allocation exists only under `--cfg heap`, which the checked crate
// receives from `.cargo/config.toml` and from nowhere else.
#[no_alloc_check::no_alloc]
#[inline(never)]
fn root(x: u8) -> u8 {
    #[cfg(heap)]
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
