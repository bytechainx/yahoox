#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! AIDD 对抗 / 边界用例（特性 005）。
//!
//! 候选由 AI 生成，逐条人工复核后仅保留「结论=保留」项；丢弃项登记于 PR 描述。
//!
//! // AIDD: 闰年边界 1900/2000/2024/2026 的二月天数 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §1 需求标准（日期严格校验） | 结论=保留
//! // AIDD: 十位长度但含非数字字符的日期串 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §6 解析与合成夹具（严格 YYYY-MM-DD） | 结论=保留
//! // AIDD: 义务集符号与禁抢主源符号的并集外输入 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §3 义务集与禁抢主源（fail-closed） | 结论=保留
//! // AIDD: 数值为 NaN / Infinity 的日线 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §1 需求标准（值必须有限） | 结论=保留
//! // AIDD: 显式 null 与字段缺省必须区分具名原因 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §6 解析与合成夹具（缺失具名，禁转 0） | 结论=保留
//! // AIDD: 重复身份（同符号同期间）必须拒绝而非静默去重 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §6 解析与合成夹具（重复身份拒绝） | 结论=保留
//! // AIDD: blocked 路径与 R3 放行路径的拒绝分类必须不同 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §5 blocked 采集路径 | 结论=保留
//! // AIDD: 授权证据仅含空白字符时不得视为有效 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §1 需求标准（fail-closed 授权） | 结论=保留

use yahoox::{
    authorize_yahoo, guard_collection_path, guard_primary_source, parse_yahoo_bars, validate_bar,
    Date, Frequency, Period, Unit, YahooAuthorization, YahooAuthorizationEvidence, YahooBar,
    YahooCollectionPath, YahooErrorKind, YahooMissingReason, YahooScope, YahooValue, DECISION_ID,
};

/// 边界：闰年规则（1900 非闰、2000 与 2024 是闰年）。
#[test]
fn leap_year_boundaries() {
    assert!(
        Date::new(2000, 2, 29).is_ok(),
        "2000 是闰年（能被 400 整除）"
    );
    assert!(
        Date::new(1900, 2, 29).is_err(),
        "1900 不是闰年（能被 100 整除）"
    );
    assert!(Date::new(2024, 2, 29).is_ok());
    assert!(Date::new(2026, 2, 29).is_err());
}

/// 边界：长度恰为 10 但含非数字字符的串不得被接受。
#[test]
fn ten_char_but_non_numeric_dates() {
    for bad in [
        "2026-08-1a",
        "202a-08-14",
        "2026-0A-14",
        "2026 08 14",
        "2026-08-1４",
    ] {
        assert!(Date::parse(bad).is_err(), "{bad} 必须被拒绝");
    }
}

/// 边界：义务集与禁抢集并集之外的符号 fail-closed。
#[test]
fn symbols_outside_both_sets_are_rejected() {
    for outsider in ["HYG", "LQD", "EEM", "GLD", "UUP", "", " gc=f"] {
        let error = guard_primary_source(outsider).expect_err("集外符号必须拒绝");
        assert_eq!(error.kind(), YahooErrorKind::Invalid, "{outsider}");
    }
    // 大小写不同即不同符号（符号是精确身份，不做归一）。
    assert!(guard_primary_source("gc=f").is_err());
}

/// 边界：NaN / Infinity 与越界排序均被拒绝。
#[test]
fn non_finite_and_inverted_values_are_rejected() {
    let mut nan = base_bar();
    nan.close = YahooValue::Present(f64::NAN);
    assert_eq!(
        validate_bar(&nan).expect_err("NaN").kind(),
        YahooErrorKind::Invalid
    );

    let mut infinite = base_bar();
    infinite.low = YahooValue::Present(f64::NEG_INFINITY);
    assert!(validate_bar(&infinite).is_err());

    let mut inverted = base_bar();
    inverted.high = YahooValue::Present(90.0);
    inverted.low = YahooValue::Present(110.0);
    assert!(validate_bar(&inverted).is_err());

    let mut open_outside = base_bar();
    open_outside.open = YahooValue::Present(120.0);
    assert!(validate_bar(&open_outside).is_err());
}

/// 边界：`null` 与缺省是两种具名缺失原因，且都不得转 0。
#[test]
fn explicit_null_differs_from_source_omitted() {
    let sample = r#"{"bars":[
        {"symbol":"GC=F","period":"2026-08-14","open":1.0,"high":2.0,"low":0.5,"close":1.5},
        {"symbol":"TLT","period":"2026-08-14","open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":null}]}"#;
    let bars = parse_yahoo_bars(sample).expect("可解析");
    assert_eq!(
        bars[0].volume,
        YahooValue::Missing(YahooMissingReason::SourceOmitted)
    );
    assert_eq!(
        bars[1].volume,
        YahooValue::Missing(YahooMissingReason::ExplicitNull)
    );
    assert_ne!(bars[0].volume, bars[1].volume);
    for bar in &bars {
        assert_eq!(bar.volume.present(), None, "缺失不得转 0");
    }
}

/// 边界：重复身份（同符号 + 同期间）必须拒绝。
#[test]
fn duplicate_identity_is_rejected_not_deduplicated() {
    let duplicate = r#"{"bars":[
        {"symbol":"CL=F","period":"2026-08-14","close":70.0},
        {"symbol":"CL=F","period":"2026-08-14","close":71.0}]}"#;
    assert_eq!(
        parse_yahoo_bars(duplicate).expect_err("重复身份").kind(),
        YahooErrorKind::Invalid
    );
    // 同符号不同期间不是重复身份。
    let distinct = r#"{"bars":[
        {"symbol":"CL=F","period":"2026-08-14","close":70.0},
        {"symbol":"CL=F","period":"2026-08-15","close":71.0}]}"#;
    assert_eq!(parse_yahoo_bars(distinct).expect("不同期间").len(), 2);
}

/// 边界：`blocked` 路径与 R3 放行路径的拒绝分类不同。
#[test]
fn blocked_and_permitted_paths_have_distinct_kinds() {
    assert_eq!(
        guard_collection_path(YahooCollectionPath::PythonYfinance)
            .expect_err("blocked")
            .kind(),
        YahooErrorKind::AuthorizationDenied
    );
    assert_eq!(
        guard_collection_path(YahooCollectionPath::YfinanceRsPublicApi)
            .expect_err("本轮不实现采集")
            .kind(),
        YahooErrorKind::NotApplicable
    );
}

/// 边界：授权证据字段为纯空白时不得视为有效。
#[test]
fn whitespace_only_evidence_is_denied() {
    for blank in ["", " ", "　"] {
        let evidence = YahooAuthorizationEvidence::new(DECISION_ID, blank, "domain");
        assert!(matches!(
            authorize_yahoo(YahooScope::OfflineParseAndTypes, Some(&evidence)),
            YahooAuthorization::Denied { .. }
        ));
    }
}

/// 构造一条义务集内的合法日线。
fn base_bar() -> YahooBar {
    YahooBar {
        symbol: "CL=F".into(),
        period: Period::Day(Date {
            year: 2026,
            month: 8,
            day: 14,
        }),
        frequency: Frequency::Daily,
        unit: Unit::FuturesContinuousQuote,
        open: YahooValue::Present(70.0),
        high: YahooValue::Present(72.0),
        low: YahooValue::Present(69.0),
        close: YahooValue::Present(71.0),
        volume: YahooValue::Present(10.0),
        revision: None,
    }
}
