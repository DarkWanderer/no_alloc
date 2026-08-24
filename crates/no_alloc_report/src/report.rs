use serde::{Deserialize, Serialize};

/// One stack frame in a violation chain, root-to-terminal order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Frame {
    /// `def_path_str` of the instance at this frame — the definition-level
    /// grouping key (generic parameters, not the concrete arguments).
    pub def_path: String,
    /// The monomorphized rendering (`instance.to_string()`), e.g.
    /// `std::boxed::Box::<i32>::new`. Kept alongside `def_path` because the
    /// analysis is per-instantiation (see ADR 0001): a chain over a generic
    /// function needs the concrete type arguments to be meaningful, and this
    /// is exactly what the stderr `via `{instance}`` notes already render,
    /// so the JSON and the diagnostic agree.
    pub instance: String,
    /// Rendered span, e.g. `src/lib.rs:12:5`; tests may normalize it to `None`.
    pub span: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Verdict {
    /// Every reachable instance bottomed out without touching the allocator.
    Pass,
    /// The chain reached an allocator terminal.
    Violation { chain: Vec<Frame> },
    /// The chain reached an edge that cannot be statically resolved
    /// (`dyn` dispatch, fn pointer, inline asm, or a MIR-less callee).
    Rejected { chain: Vec<Frame>, reason: String },
    /// A generic root has no instantiation in this crate; not a failure.
    NotInstantiated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootVerdict {
    /// `def_path_str` of the root definition (may have several instances).
    pub root: String,
    /// Rendered instance, e.g. `foo::bar::<u32>`.
    pub instance: String,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub roots: Vec<RootVerdict>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selection_errors: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReportFragment {
    pub report: Report,
    pub matched_root_specs: Vec<String>,
}

impl Report {
    pub fn is_success(&self) -> bool {
        self.selection_errors.is_empty()
            && self
                .roots
                .iter()
                .all(|r| matches!(r.verdict, Verdict::Pass | Verdict::NotInstantiated))
    }

    pub fn merge(reports: impl IntoIterator<Item = Self>) -> Self {
        let mut merged = Self::default();
        for report in reports {
            merged.roots.extend(report.roots);
            merged.selection_errors.extend(report.selection_errors);
        }
        merged.roots.sort_by(|a, b| {
            (&a.root, &a.instance, &a.verdict).cmp(&(&b.root, &b.instance, &b.verdict))
        });
        merged.roots.dedup();
        let instantiated: std::collections::HashSet<_> = merged
            .roots
            .iter()
            .filter(|root| !matches!(root.verdict, Verdict::NotInstantiated))
            .map(|root| root.root.clone())
            .collect();
        merged.roots.retain(|root| {
            !matches!(root.verdict, Verdict::NotInstantiated) || !instantiated.contains(&root.root)
        });
        merged.selection_errors.sort();
        merged.selection_errors.dedup();
        merged
    }

    /// Writes `target/no-alloc/report.json` for the test harness (and
    /// anyone else) to assert on. Creates parent directories as needed.
    pub fn write_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("json.tmp.{}", std::process::id()));
        let file = std::fs::File::create(&temporary)?;
        serde_json::to_writer_pretty(file, self).map_err(std::io::Error::other)?;
        std::fs::rename(temporary, path)
    }
}

impl ReportFragment {
    pub fn write_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("json.tmp.{}", std::process::id()));
        let file = std::fs::File::create(&temporary)?;
        serde_json::to_writer(file, self).map_err(std::io::Error::other)?;
        std::fs::rename(temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_requires_all_roots_pass_or_not_instantiated() {
        let report = Report {
            roots: vec![
                RootVerdict {
                    root: "a".into(),
                    instance: "a".into(),
                    verdict: Verdict::Pass,
                },
                RootVerdict {
                    root: "b".into(),
                    instance: "b::<u32>".into(),
                    verdict: Verdict::NotInstantiated,
                },
            ],
            selection_errors: vec![],
        };
        assert!(report.is_success());
    }

    #[test]
    fn violation_fails_the_report() {
        let report = Report {
            roots: vec![RootVerdict {
                root: "a".into(),
                instance: "a".into(),
                verdict: Verdict::Violation { chain: vec![] },
            }],
            selection_errors: vec![],
        };
        assert!(!report.is_success());
    }

    #[test]
    fn write_to_file_then_read_back() {
        let report = Report {
            roots: vec![RootVerdict {
                root: "a".into(),
                instance: "a".into(),
                verdict: Verdict::Pass,
            }],
            selection_errors: vec![],
        };
        let path = std::env::temp_dir().join(format!(
            "no_alloc_report_test_{}_{}.json",
            std::process::id(),
            line!()
        ));
        report.write_to_file(&path).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        let back: Report = serde_json::from_str(&contents).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(report.roots.len(), back.roots.len());
    }

    #[test]
    fn report_json_round_trips() {
        let report = Report {
            roots: vec![RootVerdict {
                root: "a::b".into(),
                instance: "a::b".into(),
                verdict: Verdict::Rejected {
                    chain: vec![Frame {
                        def_path: "a::b".into(),
                        instance: "a::b::<u32>".into(),
                        span: Some("src/lib.rs:1:1".into()),
                    }],
                    reason: "dyn dispatch".into(),
                },
            }],
            selection_errors: vec![],
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: Report = serde_json::from_str(&json).unwrap();
        assert_eq!(report.roots.len(), back.roots.len());
        assert!(!back.is_success());
    }

    /// `instance` and `def_path` diverge for a generic frame (concrete type
    /// arguments vs. generic parameters); the round trip must keep both,
    /// distinct, through serde.
    #[test]
    fn frame_instance_round_trips_distinctly_from_def_path() {
        let frame = Frame {
            def_path: "std::boxed::Box::<T>::new".into(),
            instance: "std::boxed::Box::<i32>::new".into(),
            span: Some("src/lib.rs:1:1".into()),
        };
        let json = serde_json::to_string(&frame).unwrap();
        assert!(json.contains("\"instance\":\"std::boxed::Box::<i32>::new\""));
        let back: Frame = serde_json::from_str(&json).unwrap();
        assert_eq!(back, frame);
        assert_ne!(back.instance, back.def_path);
    }

    #[test]
    fn merge_is_sorted_and_deduplicated() {
        let root = RootVerdict {
            root: "z".into(),
            instance: "z".into(),
            verdict: Verdict::Pass,
        };
        let merged = Report::merge([
            Report {
                roots: vec![root.clone()],
                selection_errors: vec!["b".into()],
            },
            Report {
                roots: vec![root],
                selection_errors: vec!["a".into(), "b".into()],
            },
        ]);
        assert_eq!(merged.roots.len(), 1);
        assert_eq!(merged.selection_errors, ["a", "b"]);
    }

    #[test]
    fn concrete_instance_supersedes_not_instantiated_fragment() {
        let merged = Report::merge([
            Report {
                roots: vec![RootVerdict {
                    root: "dependency::root".into(),
                    instance: "root".into(),
                    verdict: Verdict::NotInstantiated,
                }],
                selection_errors: vec![],
            },
            Report {
                roots: vec![RootVerdict {
                    root: "dependency::root".into(),
                    instance: "dependency::root::<u32>".into(),
                    verdict: Verdict::Pass,
                }],
                selection_errors: vec![],
            },
        ]);
        assert_eq!(merged.roots.len(), 1);
        assert_eq!(merged.roots[0].instance, "dependency::root::<u32>");
    }
}
