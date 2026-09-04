//! Criterion benchmarks over the pure parts of `no_alloc_report`: no
//! `TyCtxt` involved, so these run on stable and track regressions in the
//! logic every root/rejection/pass verdict passes through.

use criterion::{criterion_group, criterion_main, Criterion};
use no_alloc_report::{
    parse_root_spec, Environment, Frame, PanicStrategy, Report, RootVerdict, Verdict,
};

fn deep_report(depth: usize) -> Report {
    let chain = (0..depth)
        .map(|i| Frame {
            def_path: format!("crate_{i}::module_{i}::function_{i}"),
            instance: format!("crate_{i}::module_{i}::function_{i}::<u32>"),
            span: Some(format!("src/lib_{i}.rs:{i}:1")),
        })
        .collect();
    Report {
        roots: vec![RootVerdict {
            root: "root".into(),
            instance: "root".into(),
            verdict: Verdict::Violation { chain },
            environment: Some(Environment {
                panic_strategy: PanicStrategy::Abort,
                opt_level: "No".into(),
                mir_opt_level: 1,
                target_triple: "x86_64-unknown-linux-gnu".into(),
                rustc_version: "1.99.0-nightly".into(),
                all_crates: false,
                build_std: false,
            }),
        }],
        panic_strategy: Some(no_alloc_report::PanicStrategy::Abort),
        ..Report::default()
    }
}

fn bench_root_spec_parsing(c: &mut Criterion) {
    let spec = (0..64)
        .map(|i| format!("mycrate::module_{i}::function_{i}"))
        .collect::<Vec<_>>()
        .join(",");
    c.bench_function("parse_root_spec/64_entries", |b| {
        b.iter(|| parse_root_spec(std::hint::black_box(&spec)))
    });
}

fn bench_report_serialize(c: &mut Criterion) {
    let report = deep_report(64);
    c.bench_function("report/serialize_64_frame_chain", |b| {
        b.iter(|| serde_json::to_string(std::hint::black_box(&report)).unwrap())
    });
}

fn bench_report_round_trip(c: &mut Criterion) {
    let report = deep_report(64);
    let json = serde_json::to_string(&report).unwrap();
    c.bench_function("report/deserialize_64_frame_chain", |b| {
        b.iter(|| serde_json::from_str::<Report>(std::hint::black_box(&json)).unwrap())
    });
}

fn bench_report_merge(c: &mut Criterion) {
    let reports: Vec<_> = (0..64).map(|_| deep_report(8)).collect();
    c.bench_function("report/merge_64_reports", |b| {
        b.iter(|| Report::merge(std::hint::black_box(reports.clone())))
    });
}

criterion_group!(
    benches,
    bench_root_spec_parsing,
    bench_report_serialize,
    bench_report_round_trip,
    bench_report_merge
);
criterion_main!(benches);
