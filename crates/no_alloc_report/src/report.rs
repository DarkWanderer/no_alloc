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

/// The panic strategy the checked build was compiled under.
///
/// A `Pass` means something different in each of these, so a report that
/// does not record it cannot be read back correctly once the invocation
/// that produced it is gone (ADR 0006). This is the strategy rustc actually
/// compiled with, not the flag the user typed: `RUSTFLAGS` can set it too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanicStrategy {
    /// `Assert` rejects outright; nothing panic-adjacent is checkable.
    Unwind,
    /// `Assert` is out of the guarantee's scope — not proven free of
    /// allocation, since a panic still calls the handler (ADR 0003).
    Abort,
    /// Panic paths lower to a bare `abort()` and are traversed like any
    /// other edge, so a `Pass` covers them too.
    ImmediateAbort,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub roots: Vec<RootVerdict>,
    /// `None` when nothing recorded one (an empty run), or when fragments
    /// disagreed — which one build should never produce, and is reported as
    /// "unknown" rather than by picking a winner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panic_strategy: Option<PanicStrategy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selection_errors: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReportFragment {
    pub report: Report,
    pub matched_root_specs: Vec<String>,
}

impl Report {
    /// Whether at least one root in this report was actually checked — that
    /// is, has a verdict other than `NotInstantiated`. Used to decide
    /// whether a report has anything to say about the panic strategy it was
    /// compiled under: a fragment holding only `NotInstantiated` markers (a
    /// cross-crate generic's defining crate, or a wrapped host unit that
    /// merely rediscovered an uninstantiated root) checked nothing, so it
    /// cannot confirm or contradict a sibling's claim either way.
    pub fn checked_an_instance(&self) -> bool {
        self.roots
            .iter()
            .any(|root| !matches!(root.verdict, Verdict::NotInstantiated))
    }

    pub fn is_success(&self) -> bool {
        self.selection_errors.is_empty()
            && self
                .roots
                .iter()
                .all(|r| matches!(r.verdict, Verdict::Pass | Verdict::NotInstantiated))
    }

    pub fn merge(reports: impl IntoIterator<Item = Self>) -> Self {
        let mut merged = Self::default();
        let mut strategies = std::collections::BTreeSet::new();
        // A report that carries verdicts but no strategy makes the result
        // unknowable, and must not be quietly relabelled by a sibling that
        // does carry one — that would put a strategy on roots whose panic
        // semantics nobody recorded (a legacy report, or an earlier merge
        // that already found a conflict). Keeping it also makes `merge`
        // associative: nested merges give the same answer as one flat one.
        //
        // "Carries verdicts" means a root that was actually checked, the
        // same test the driver applies before claiming a strategy at all. A
        // `NotInstantiated` root records a marker nobody checked here, so a
        // fragment holding only those has nothing to say about the strategy
        // in either direction — and a cross-crate generic produces exactly
        // such a fragment in the defining crate on every ordinary build.
        let mut unknown_over_verdicts = false;
        for report in reports {
            if report.panic_strategy.is_none() && report.checked_an_instance() {
                unknown_over_verdicts = true;
            }
            merged.roots.extend(report.roots);
            merged.selection_errors.extend(report.selection_errors);
            strategies.extend(report.panic_strategy);
        }
        // Every fragment of one build sees the same flags, so anything other
        // than exactly one answer means the report cannot claim a strategy.
        let mut found = strategies.into_iter();
        merged.panic_strategy = match (found.next(), found.next()) {
            (single, None) if !unknown_over_verdicts => single,
            _ => None,
        };
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
            panic_strategy: None,
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
            panic_strategy: None,
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
            panic_strategy: None,
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
            panic_strategy: None,
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

    /// One build compiles everything with one panic strategy, so the merged
    /// report can state it. Anything else is "unknown" rather than a guess:
    /// a `Pass` read back later means different things per strategy.
    #[test]
    fn checked_an_instance_ignores_not_instantiated_markers() {
        let checked = Report {
            roots: vec![RootVerdict {
                root: "a".into(),
                instance: "a".into(),
                verdict: Verdict::Pass,
            }],
            ..Report::default()
        };
        assert!(checked.checked_an_instance());
        let only_markers = Report {
            roots: vec![RootVerdict {
                root: "dependency::root".into(),
                instance: "dependency::root".into(),
                verdict: Verdict::NotInstantiated,
            }],
            ..Report::default()
        };
        assert!(!only_markers.checked_an_instance());
        assert!(!Report::default().checked_an_instance());
    }

    #[test]
    fn merge_keeps_an_agreed_panic_strategy_and_drops_a_disagreement() {
        let with = |strategy| Report {
            panic_strategy: strategy,
            ..Report::default()
        };
        assert_eq!(
            Report::merge([
                with(Some(PanicStrategy::ImmediateAbort)),
                with(Some(PanicStrategy::ImmediateAbort)),
            ])
            .panic_strategy,
            Some(PanicStrategy::ImmediateAbort)
        );
        assert_eq!(
            Report::merge([with(Some(PanicStrategy::Abort)), with(None)]).panic_strategy,
            Some(PanicStrategy::Abort)
        );
        assert_eq!(
            Report::merge([
                with(Some(PanicStrategy::Abort)),
                with(Some(PanicStrategy::Unwind)),
            ])
            .panic_strategy,
            None
        );
        assert_eq!(Report::merge([]).panic_strategy, None);
    }

    /// A report with verdicts and no strategy is not the same as a rootless
    /// fragment: the first says "these roots' panic semantics are
    /// unrecorded", and letting a sibling supply one would put a label on
    /// verdicts that never had it. It also keeps `merge` associative.
    #[test]
    fn merge_does_not_relabel_verdicts_whose_strategy_is_unknown() {
        let root = RootVerdict {
            root: "a".into(),
            instance: "a".into(),
            verdict: Verdict::Pass,
        };
        let known = Report {
            roots: vec![root.clone()],
            panic_strategy: Some(PanicStrategy::ImmediateAbort),
            ..Report::default()
        };
        let unknown_with_roots = Report {
            roots: vec![root],
            panic_strategy: None,
            ..Report::default()
        };
        // A legacy or already-conflicted report poisons the claim...
        assert_eq!(
            Report::merge([known.clone(), unknown_with_roots.clone()]).panic_strategy,
            None
        );
        // ...while a rootless fragment (a wrapped build script) does not.
        assert_eq!(
            Report::merge([known.clone(), Report::default()]).panic_strategy,
            Some(PanicStrategy::ImmediateAbort)
        );
        // Nor does one holding only `NotInstantiated` markers, which is what
        // the defining crate of a cross-crate generic contributes on every
        // ordinary build (`tests/ui/cross_crate_generic`).
        let only_markers = Report {
            roots: vec![RootVerdict {
                root: "dependency::root".into(),
                instance: "dependency::root".into(),
                verdict: Verdict::NotInstantiated,
            }],
            panic_strategy: None,
            ..Report::default()
        };
        assert_eq!(
            Report::merge([known.clone(), only_markers]).panic_strategy,
            Some(PanicStrategy::ImmediateAbort)
        );
        // Associativity: nesting the same inputs gives the same answer.
        let nested = Report::merge([Report::merge([known.clone(), unknown_with_roots.clone()])]);
        assert_eq!(nested.panic_strategy, None);
        assert_eq!(
            nested.panic_strategy,
            Report::merge([known, unknown_with_roots]).panic_strategy
        );
    }

    /// The field is skipped when absent, so a report written before it
    /// existed still parses — and one written now round-trips its strategy.
    #[test]
    fn panic_strategy_round_trips_and_is_optional() {
        let json = serde_json::to_string(&Report {
            panic_strategy: Some(PanicStrategy::ImmediateAbort),
            ..Report::default()
        })
        .unwrap();
        assert!(json.contains("\"immediate_abort\""), "{json}");
        assert_eq!(
            serde_json::from_str::<Report>(&json)
                .unwrap()
                .panic_strategy,
            Some(PanicStrategy::ImmediateAbort)
        );
        let legacy: Report = serde_json::from_str(r#"{"roots":[]}"#).unwrap();
        assert_eq!(legacy.panic_strategy, None);
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
                panic_strategy: None,
                selection_errors: vec!["b".into()],
            },
            Report {
                roots: vec![root],
                panic_strategy: None,
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
                panic_strategy: None,
                selection_errors: vec![],
            },
            Report {
                roots: vec![RootVerdict {
                    root: "dependency::root".into(),
                    instance: "dependency::root::<u32>".into(),
                    verdict: Verdict::Pass,
                }],
                panic_strategy: None,
                selection_errors: vec![],
            },
        ]);
        assert_eq!(merged.roots.len(), 1);
        assert_eq!(merged.roots[0].instance, "dependency::root::<u32>");
    }
}
