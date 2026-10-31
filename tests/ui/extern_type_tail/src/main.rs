#![feature(extern_types)]
unsafe extern "C" {
    pub type Opaque;
}
#[repr(C)]
pub struct S {
    a: u32,
    tail: Opaque,
}

#[no_alloc_check::no_alloc]
#[inline(never)]
pub fn root(s: &S) -> *const Opaque {
    &raw const s.tail
}

fn main() {
    let x = [0u64; 2];
    let s = unsafe { &*(x.as_ptr() as *const S) };
    println!("{:p} {}", root(s), s.a);
}
