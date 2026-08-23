use serde::{Deserialize, Serialize};

/// Where per-crate root sidecars are written, relative to the workspace
/// target dir: `target/no-alloc/roots/<crate>.json`.
pub const SIDECAR_DIR: &str = "no-alloc/roots";

/// Mirrors `rustc_data_structures::stable_hasher::Fingerprint`, the payload
/// of `rustc_hir::def_id::DefPathHash`. Stable and cross-crate comparable,
/// so this is exact-match, not a string heuristic. Kept as a plain (u64, u64)
/// here so this crate stays independent of rustc internals; `no_alloc_analysis`
/// owns the `From`/`Into` conversion to the real `DefPathHash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DefPathHash(pub u64, pub u64);

/// One workspace crate's local roots, written by the driver on every run and
/// unioned by the leaf crate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RootIndex {
    pub crate_name: String,
    pub roots: Vec<DefPathHash>,
}

impl RootIndex {
    pub fn file_name(crate_name: &str) -> String {
        format!("{crate_name}.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_index_json_round_trips() {
        let index = RootIndex {
            crate_name: "mycrate".into(),
            roots: vec![DefPathHash(1, 2), DefPathHash(3, 4)],
        };
        let json = serde_json::to_string(&index).unwrap();
        let back: RootIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(index.crate_name, back.crate_name);
        assert_eq!(index.roots, back.roots);
    }

    #[test]
    fn file_name_matches_crate() {
        assert_eq!(RootIndex::file_name("mycrate"), "mycrate.json");
    }
}
