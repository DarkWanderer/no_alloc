//! Regression fixture for F0 (soundness review): dropping a `dyn Trait`
//! place directly (no `Box`, no global allocator/deallocator anywhere in
//! the chain) must be rejected, because the concrete destructor behind the
//! vtable is not statically known. Unlike `drop_dyn`, there is nothing here
//! that could mask a missing `dyn`-drop edge with an unrelated violation.

trait Greet {
    #[allow(dead_code)]
    fn greet(&self) -> i32;
}

struct En;

impl Greet for En {
    fn greet(&self) -> i32 {
        1
    }
}

impl Drop for En {
    fn drop(&mut self) {
        // The concrete destructor. Erased by the time drop glue for `dyn
        // Greet` runs, so the checker cannot see whether this allocates --
        // it must reject, not silently pass it through.
    }
}

#[no_alloc_check::no_alloc]
fn root(value: *mut dyn Greet) {
    unsafe {
        std::ptr::drop_in_place(value);
    }
}

fn main() {
    let mut en = En;
    root(&mut en as *mut dyn Greet);
}
