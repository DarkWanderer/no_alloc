#[no_alloc_check::no_alloc]
#[inline(never)]
fn root(x: u8) -> u8 {
    #[cfg(heap)]
    {
        *Box::new(x)
    }
    #[cfg(not(heap))]
    {
        x
    }
}

fn main() {
    println!("{}", root(1));
}
