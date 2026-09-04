pub fn allocates() -> usize {
    *Box::new(1)
}

pub fn pure() -> usize {
    1
}
