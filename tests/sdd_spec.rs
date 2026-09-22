#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! SDD 规格对照（特性 005）：把 `docs/标准.md` 的每个 `##` 章节转成可执行断言。
//!
//! 章节与断言函数须与 `docs/标准.md` 的 `##` 章节 1:1（检查器按标题逐字比对）。
//!
//! // SPEC-MAP: S-1 | 1. 需求标准 | assert_requirements_standard
//! // SPEC-MAP: S-2 | 2. 设计标准 | assert_design_standard
//! // SPEC-MAP: S-3 | 3. 义务集与禁抢主源 | assert_obligation_and_forbidden_sets
//! // SPEC-MAP: S-4 | 4. 禁静默替换 | assert_non_interchangeable_pairs
//! // SPEC-MAP: S-5 | 5. blocked 采集路径 | assert_blocked_collection_paths
//! // SPEC-MAP: S-6 | 6. 解析与合成夹具 | assert_parse_and_synthetic_fixtures
//! // SPEC-MAP: S-7 | 7. publication 语义 | assert_publication_semantics
//! // SPEC-MAP: S-8 | 8. 非目标与门禁 | assert_non_goals_and_gates

use yahoox::{
    authorize_yahoo, guard_collection_path, guard_non_interchangeable, guard_primary_source,
    is_collection_obligation, is_forbidden_primary, is_formal_pit_eligible, parse_yahoo_bars,
    publication_semantics, unit_for_symbol, validate_bar, AvailabilityEvidence, Date, Frequency,
    Period, PitEligibility, TimePrecision, Unit, YahooAuthorization, YahooAuthorizationEvidence,
    YahooBar, YahooCollectionPath, YahooErrorKind, YahooMissingReason, YahooScope, YahooValue,
    ACTIVE_COLLECTION_OBLIGATIONS, DECISION_ID, FORBIDDEN_PRIMARY_SYMBOLS,
    NON_INTERCHANGEABLE_PAIRS,
};

/// S-1：值对象、守卫、解析器、授权判定与 publication 语义五类能力均可达。
#[test]
fn assert_requirements_standard() {
    // 值对象：日期 / 期间 / 频率 / 口径均可用。
    let date = Date::parse("2026-08-14").expect("合法日期");
    assert_eq!(
        Period::parse_day("2026-08-14").expect("合法日"),
        Period::Day(date)
    );
    assert_eq!(Frequency::Daily, Frequency::Daily);
    assert_eq!(
        unit_for_symbol("TLT").expect("义务集符号"),
        Unit::EtfShareQuote
    );

    // 守卫：义务集放行、禁抢主源拒绝。
    assert!(guard_primary_source("TLT").is_ok());
    assert!(guard_primary_source("^VIX").is_err());

    // 解析器与授权判定可达。
    assert_eq!(
        parse_yahoo_bars(r#"{"bars":[]}"#)
            .expect("空集合合法")
            .len(),
        0
    );
    assert!(matches!(
        authorize_yahoo(YahooScope::OfflineParseAndTypes, None),
        YahooAuthorization::Denied { .. }
    ));

    // 错误统一为 YahooError，各失败语义各有变体，不靠字符串分类。
    for error in [
        yahoox::YahooError::Invalid("x".into()),
        yahoox::YahooError::Missing("x".into()),
        yahoox::YahooError::AuthorizationDenied("x".into()),
        yahoox::YahooError::RoutedElsewhere("x".into()),
        yahoox::YahooError::WriteAuthorityDenied("x".into()),
        yahoox::YahooError::SemanticallyRejected("x".into()),
        yahoox::YahooError::NotApplicable("x".into()),
        yahoox::YahooError::Invariant("x".into()),
    ] {
        assert!(!error.to_string().is_empty(), "错误须可展示：{error:?}");
    }
}

/// S-2：模块切分（error / value / value::symbols / value::bar / authz / pit / parse）
/// 体现在公开路径上，依赖方向单向。
#[test]
fn assert_design_standard() {
    // value 面：基础值对象经 crate 根可达。
    assert_eq!(
        Date::new(2026, 8, 14).expect("合法"),
        Date {
            year: 2026,
            month: 8,
            day: 14
        }
    );

    // symbols 面：常量与守卫经 crate 根可达。
    assert_eq!(ACTIVE_COLLECTION_OBLIGATIONS.len(), 11);
    assert!(is_forbidden_primary("^TNX"));

    // bar 面：观测类型与校验入口经 crate 根可达。
    let bar = YahooBar {
        symbol: "DX-Y.NYB".into(),
        period: Period::Day(date_of(2026, 8, 14)),
        frequency: Frequency::Daily,
        unit: Unit::DxyPoints,
        open: YahooValue::Present(98.0),
        high: YahooValue::Present(99.0),
        low: YahooValue::Present(97.5),
        close: YahooValue::Present(98.6),
        volume: YahooValue::Missing(YahooMissingReason::SourceOmitted),
        revision: None,
    };
    assert!(validate_bar(&bar).is_ok());

    // authz / pit 面。
    assert_eq!(
        YahooAuthorizationEvidence::new(DECISION_ID, "ZoneCNH", "s").decision_id,
        DECISION_ID
    );
    assert_eq!(publication_semantics().0, TimePrecision::Date);
}

/// S-3：义务集 11 符与禁抢主源 9 符的取值与判据。
#[test]
fn assert_obligation_and_forbidden_sets() {
    assert_eq!(
        ACTIVE_COLLECTION_OBLIGATIONS,
        [
            "GC=F", "HG=F", "CL=F", "RSP", "TLT", "FEZ", "^STOXX", "IWM", "^RUT", "DX-Y.NYB",
            "USDCNH=X"
        ]
    );
    assert_eq!(
        FORBIDDEN_PRIMARY_SYMBOLS,
        ["^GSPC", "^VIX", "^IXIC", "^N225", "JPY=X", "USDJPY=X", "^TNX", "BTC-USD", "ETH-USD"]
    );
    for symbol in FORBIDDEN_PRIMARY_SYMBOLS {
        let error = guard_primary_source(symbol).expect_err("禁抢主源必须拒绝");
        assert_eq!(error.kind(), YahooErrorKind::RoutedElsewhere);
    }
    for symbol in ACTIVE_COLLECTION_OBLIGATIONS {
        assert!(guard_primary_source(symbol).is_ok());
        assert!(is_collection_obligation(symbol));
    }
}

/// S-4：21 条禁替换对登记齐备，且守卫左右对称。
#[test]
fn assert_non_interchangeable_pairs() {
    assert_eq!(NON_INTERCHANGEABLE_PAIRS.len(), 21);
    for required in [
        ("^GSPC", "SPY"),
        ("^IXIC", "QQQ"),
        ("^IXIC", "^NDX"),
        ("^N225", "EWJ"),
        ("RSP", "^RUT"),
        ("RSP", "SPY"),
        ("TLT", "^TNX"),
        ("CL=F", "BZ=F"),
        ("^STOXX", "^STOXX50E"),
        ("USDCNH=X", "USDCNY=X"),
        ("DX-Y.NYB", "DTWEXBGS"),
        ("JPY=X", "DEXJPUS"),
        ("USDJPY=X", "DEXJPUS"),
        ("^TNX", "DGS10"),
    ] {
        assert_eq!(
            guard_non_interchangeable(required.0, required.1)
                .expect_err("禁替换对必须拒绝")
                .kind(),
            YahooErrorKind::SemanticallyRejected
        );
        assert!(guard_non_interchangeable(required.1, required.0).is_err());
    }
    for pair in NON_INTERCHANGEABLE_PAIRS {
        assert!(!pair.difference.is_empty(), "差异说明不得为空");
    }
}

/// S-5：四个 `blocked` 路径均拒绝；R3 放行路径仍不实现采集。
#[test]
fn assert_blocked_collection_paths() {
    for path in [
        YahooCollectionPath::PythonYfinance,
        YahooCollectionPath::CookieCrumbSelfBuilt,
        YahooCollectionPath::ProxyRotationOrChallengeBypass,
        YahooCollectionPath::FredPrimaryUsurpation,
    ] {
        assert_eq!(
            guard_collection_path(path).expect_err("blocked").kind(),
            YahooErrorKind::AuthorizationDenied,
            "{path:?}"
        );
    }
    assert_eq!(
        guard_collection_path(YahooCollectionPath::YfinanceRsPublicApi)
            .expect_err("本轮不实现采集")
            .kind(),
        YahooErrorKind::NotApplicable
    );
}

/// S-6：白名单原子失败、日期严格、重复身份拒绝、缺失具名。
#[test]
fn assert_parse_and_synthetic_fixtures() {
    let sample = include_str!("fixtures/bars.json");
    let bars = parse_yahoo_bars(sample).expect("合成样本可解析");
    assert_eq!(bars.len(), 3);
    assert!(
        sample.contains("\"_synthetic\": true"),
        "夹具必须自带合成标注"
    );
    assert!(sample.contains("不构成任何证据"));

    // 未知字段 → 原子失败。
    let unknown = r#"{"bars":[{"symbol":"GC=F","period":"2026-08-14","close":1.0,"v":2}]}"#;
    assert_eq!(
        parse_yahoo_bars(unknown).expect_err("未知字段").kind(),
        YahooErrorKind::SemanticallyRejected
    );
    // 日期不严格。
    assert!(
        parse_yahoo_bars(r#"{"bars":[{"symbol":"GC=F","period":"2026-8-14","close":1.0}]}"#)
            .is_err()
    );
    // 重复身份 → 拒绝（不去重）。
    assert_eq!(
        parse_yahoo_bars(include_str!("fixtures/duplicate_identity.json"))
            .expect_err("重复身份")
            .kind(),
        YahooErrorKind::Invalid
    );
    // 缺失具名，不转 0。
    assert_eq!(
        YahooValue::Missing(YahooMissingReason::ExplicitNull).present(),
        None
    );
}

/// S-7：publication 三元组恒为 `(Date, Inferred, NotEligible)`。
#[test]
fn assert_publication_semantics() {
    assert_eq!(
        publication_semantics(),
        (
            TimePrecision::Date,
            AvailabilityEvidence::Inferred,
            PitEligibility::NotEligible
        )
    );
    assert!(!is_formal_pit_eligible());
}

/// S-8：非目标与门禁——不联网、不读凭据、不依赖兄弟仓，生产结论为 NO-GO。
#[test]
fn assert_non_goals_and_gates() {
    // 解析器只吃字符串：没有任何网络参数可传。
    let bars =
        parse_yahoo_bars(r#"{"bars":[{"symbol":"GC=F","period":"2026-08-14","close":1.0}]}"#)
            .expect("可解析");
    assert_eq!(bars.len(), 1);

    // 单 crate 独立：本 crate 无 path 依赖，且依赖段只允许白名单三项。
    let manifest = include_str!("../Cargo.toml");
    assert!(!manifest.contains("path = \"../"), "不得有跨仓 path 依赖");
    let deps: Vec<&str> = manifest
        .lines()
        .skip_while(|line| *line != "[dependencies]")
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(
        deps.len(),
        3,
        "依赖段只允许 thiserror / serde / serde_json，实际：{deps:?}"
    );

    // 零端点字面量 / 零凭据读取：**运行期递归枚举** `src/` 下全部 `.rs`
    // （以 `CARGO_MANIFEST_DIR` 为根，不依赖 cwd；日后新增 src 文件自动纳入，不用手写清单）。
    let sources = collect_src_sources();
    assert!(!sources.is_empty(), "src/ 下必须至少有一个 .rs 源文件");
    for (path, text) in &sources {
        for (needle, why) in [
            ("http://", "不得含 http:// 端点字面量"),
            ("https://", "不得含 https:// 端点字面量"),
            ("env::var", "不得读取环境变量"),
            ("from_env", "不得读取环境变量/凭据"),
        ] {
            assert!(
                !text.contains(needle),
                "{} 含 {needle:?}：{why}",
                path.display()
            );
        }
    }

    // 生产结论：本层不声称任何能力等级。
    assert_eq!(bars[0].revision, None, "不得伪造修订标识");
}

/// 递归枚举 `src/` 下全部 `.rs` 的源码文本（不依赖 cwd，新增文件自动纳入）。
fn collect_src_sources() -> Vec<(std::path::PathBuf, String)> {
    /// 递归收集 `.rs` 路径。
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension() == Some(std::ffi::OsStr::new("rs")) {
                out.push(path);
            }
        }
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).expect("src 源码可读");
            (path, text)
        })
        .collect()
}

/// 构造一个日期（测试辅助）。
fn date_of(year: i16, month: u8, day: u8) -> Date {
    Date { year, month, day }
}
