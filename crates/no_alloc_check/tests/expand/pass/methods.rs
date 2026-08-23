#![allow(unexpected_cfgs)]

struct Counter(u32);

impl Counter {
    #[no_alloc_check::no_alloc]
    fn get(&self) -> u32 {
        self.0
    }
}

trait Read {
    fn read(&self) -> u32;
}

impl Read for Counter {
    #[no_alloc_check::no_alloc]
    fn read(&self) -> u32 {
        self.0
    }
}

fn main() {
    let counter = Counter(1);
    assert_eq!(counter.get(), counter.read());
}
