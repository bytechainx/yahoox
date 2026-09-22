#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! yahoox 热路径：合成样本解析 + 义务集守卫。
//!
//! `harness = false`；`--quick` 走小迭代数，便于本地快速自检。
use std::hint::black_box;
use std::time::Instant;

use yahoox::{guard_primary_source, parse_yahoo_bars};

/// 合成样本（内联，避免 bench 依赖文件读取）。
const SAMPLE: &str = r#"{
    "bars": [
        {"symbol":"GC=F","period":"2026-08-14","open":2450.5,"high":2471.0,
         "low":2440.2,"close":2466.8,"volume":123456},
        {"symbol":"USDCNH=X","period":"2026-08-14","open":7.112,"high":7.166,
         "low":7.094,"close":7.141,"volume":null},
        {"symbol":"^STOXX","period":"2026-08-14","open":520.1,"high":523.4,
         "low":518.7,"close":522.0,"volume":null}
    ]
}"#;

fn iters() -> u32 {
    if std::env::args().any(|arg| arg == "--quick") {
        200
    } else {
        5_000
    }
}

fn main() {
    let n = iters();
    for _ in 0..n.min(10) {
        let _ = parse_yahoo_bars(SAMPLE).expect("合成样本可解析");
    }
    let start = Instant::now();
    let mut parsed = 0usize;
    for _ in 0..n {
        let bars = parse_yahoo_bars(SAMPLE).expect("合成样本可解析");
        parsed = parsed.wrapping_add(bars.len());
        black_box(guard_primary_source("^GSPC").is_err());
    }
    let elapsed = start.elapsed();
    println!(
        "bench_yahoox_parse: iters={n} total={elapsed:?} per_iter={:?} bars={}",
        elapsed / n,
        black_box(parsed)
    );
}
