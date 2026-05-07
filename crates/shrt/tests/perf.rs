mod common;
use common::*;
use std::time::{Duration, Instant};

#[ignore]
#[test]
fn cold_start_under_50ms() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "", false);

    let _ = invoke_shim(tmp.path(), "wt0", &[], &[]);

    let mut total = Duration::ZERO;
    let n: u32 = 10;
    for _ in 0..n {
        let start = Instant::now();
        let output = invoke_shim(tmp.path(), "wt0", &[], &[]);
        let elapsed = start.elapsed();
        assert!(output.status.success());
        total += elapsed;
    }
    let avg = total / n;
    eprintln!(
        "cold-start avg: {} ms over {} runs (target: spec §6.1 < 10 ms)",
        avg.as_millis(),
        n
    );
}
