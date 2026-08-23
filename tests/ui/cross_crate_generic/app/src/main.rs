struct Pure;

impl generic_dep::Work for Pure {
    fn work() -> usize { 1 }
}

struct Allocates;

impl generic_dep::Work for Allocates {
    fn work() -> usize { *Box::new(1) }
}

fn main() {
    println!("{}", generic_dep::root::<Pure>());
    println!("{}", generic_dep::root::<Allocates>());
}
