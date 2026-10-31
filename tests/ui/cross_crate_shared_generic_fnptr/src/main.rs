fn main() {
    let f: fn() -> usize = std::hint::black_box(shared_dep::process::<u32>);
    println!("{}", f());
}
