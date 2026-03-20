use criterion::{black_box, criterion_group, criterion_main, Criterion};
use laddermd_core::parser;

fn fixture(name: &str) -> String {
    let path = format!("../../tests/fixtures/{name}");
    std::fs::read_to_string(&path).unwrap()
}

fn bench_parse_all(c: &mut Criterion) {
    let fixtures = [
        ("self_hold", fixture("self_hold.xml")),
        ("interlock", fixture("interlock.xml")),
        ("timer", fixture("timer.xml")),
        ("emergency_stop", fixture("emergency_stop.xml")),
        ("counter", fixture("counter.xml")),
        ("comparison", fixture("comparison.xml")),
    ];

    for (name, xml) in &fixtures {
        c.bench_function(&format!("parse_{name}"), |b| {
            b.iter(|| parser::parse(black_box(xml)).unwrap())
        });
    }
}

criterion_group!(benches, bench_parse_all);
criterion_main!(benches);
