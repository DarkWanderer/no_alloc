use std::alloc::{GlobalAlloc, Layout, System};

struct MyAlloc;
unsafe impl GlobalAlloc for MyAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: MyAlloc = MyAlloc;

#[no_alloc::no_alloc]
fn root() -> i32 {
    *Box::new(9)
}

fn main() {
    println!("{}", root());
}
