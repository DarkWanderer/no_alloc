/// Parses the `NO_ALLOC_ROOTS` fallback env var: a comma-separated list of
/// `def_path_str`-shaped function paths, e.g. `mycrate::path::fn,other::fn`.
///
/// This is matched against `def_path_str` directly (not `DefPathHash`) since
/// it names things that may not have been compiled yet (third-party code the
/// user cannot annotate).
pub fn parse_root_spec(spec: &str) -> Vec<String> {
    spec.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_yields_no_roots() {
        assert_eq!(parse_root_spec(""), Vec::<String>::new());
    }

    #[test]
    fn splits_and_trims_entries() {
        assert_eq!(
            parse_root_spec(" mycrate::a::f , mycrate::b::g,,"),
            vec!["mycrate::a::f", "mycrate::b::g"]
        );
    }

    #[test]
    fn single_entry_no_trailing_comma() {
        assert_eq!(parse_root_spec("mycrate::f"), vec!["mycrate::f"]);
    }
}
