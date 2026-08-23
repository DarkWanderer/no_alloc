use serde::{Deserialize, Serialize};

/// One stack frame in a violation chain, root-to-terminal order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    /// `def_path_str` of the instance at this frame.
    pub def_path: String,
    /// Rendered span, e.g. `src/lib.rs:12:5`. `None` before M5 wires up spans.
    pub span: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl Report {
    pub fn is_success(&self) -> bool {
        self.roots
            .iter()
            .all(|r| matches!(r.verdict, Verdict::Pass | Verdict::NotInstantiated))
    }

    /// Writes `target/no-alloc/report.json` for the test harness (and
    /// anyone else) to assert on. Creates parent directories as needed.
    pub fn write_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self).map_err(std::io::Error::other)
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
                        span: Some("src/lib.rs:1:1".into()),
                    }],
                    reason: "dyn dispatch".into(),
                },
            }],
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: Report = serde_json::from_str(&json).unwrap();
        assert_eq!(report.roots.len(), back.roots.len());
        assert!(!back.is_success());
    }
}
