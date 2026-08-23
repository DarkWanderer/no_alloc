struct Holder(String);

#[no_alloc::no_alloc]
fn root(h: Holder) {
    let _ = h;
}

fn main() {
    root(Holder(String::from("hi")));
}
