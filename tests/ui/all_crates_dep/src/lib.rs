//! Half of the F5 (soundness review) fixture for `--all-crates`: a marked,
//! allocating function that lives in a crate `all_crates_marker` depends on
//! via a plain path dependency, but which is *not* a member of
//! `all_crates_marker`'s own workspace (each `tests/ui/<case>` fixture
//! declares its own empty `[workspace]` table, and this crate is a sibling
//! directory, not nested under `all_crates_marker`'s). Also a valid,
//! ordinary standalone case on its own — running the checker directly here
//! finds the same violation `direct_alloc` does.

#[no_alloc_check::no_alloc]
pub fn dep_allocates() -> usize {
    *Box::new(1)
}
