trait MaybeAlloc {
    fn get(x: i32) -> i32;
}

struct AllocKind;
impl MaybeAlloc for AllocKind {
    fn get(x: i32) -> i32 {
        *Box::new(x)
    }
}

struct PureKind;
impl MaybeAlloc for PureKind {
    fn get(x: i32) -> i32 {
        x
    }
}

#[no_alloc::no_alloc]
fn root<K: MaybeAlloc>() -> i32 {
    K::get(41)
}

fn main() {
    println!("{} {}", root::<AllocKind>(), root::<PureKind>());
}
