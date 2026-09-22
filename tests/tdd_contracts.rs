#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! TDD 行为契约（特性 005）。
//!
//! 入口集合 = 本 crate 全部公开入口（含 `validate*` / 判定函数 / 解析器）。
//! 下表每个入口先在 `/tmp` 变异副本上观测应红、再在本树观测绿。
//!
//! 全部用例离线运行：样本为 `tests/fixtures/` 内的合成样本（经 `include_str!` 内联），
//! 不依赖任何外部服务，不读环境变量。
//!
//! // TDD-PROBE: YahooError::kind | 变异：某个变体的分类映射改指相邻变体 | 红=error_kind_maps_each_variant | 绿=error_kind_maps_each_variant
//! // TDD-PROBE: YahooError::is_retryable | 变异：改为除 Invariant 外全部返回 true | 红=error_is_retryable_only_for_invariant | 绿=error_is_retryable_only_for_invariant
//! // TDD-PROBE: Date::new | 变异：去掉闰年判定，二月恒为 29 天 | 红=date_new_validates_range | 绿=date_new_validates_range
//! // TDD-PROBE: Date::parse | 变异：放宽长度校验，接受 2026-2-3 | 红=date_parse_is_strict_iso | 绿=date_parse_is_strict_iso
//! // TDD-PROBE: Period::parse_day | 变异：返回 Period::Year 而非 Period::Day | 红=period_parse_day_is_single_day | 绿=period_parse_day_is_single_day
//! // TDD-PROBE: is_collection_obligation | 变异：把 FORBIDDEN_PRIMARY_SYMBOLS 也判为义务 | 红=collection_obligation_set_is_the_eleven | 绿=collection_obligation_set_is_the_eleven
//! // TDD-PROBE: is_forbidden_primary | 变异：只匹配前 8 个禁抢符号 | 红=forbidden_primary_set_is_the_nine | 绿=forbidden_primary_set_is_the_nine
//! // TDD-PROBE: unit_for_symbol | 变异：把 DX-Y.NYB 的报价口径改为 IndexPoints | 红=unit_for_symbol_follows_the_manifest | 绿=unit_for_symbol_follows_the_manifest
//! // TDD-PROBE: guard_primary_source | 变异：禁抢主源改为返回 Ok | 红=guard_primary_source_rejects_usurpers | 绿=guard_primary_source_rejects_usurpers
//! // TDD-PROBE: guard_non_interchangeable | 变异：只按原顺序匹配、忽略左右互换 | 红=guard_non_interchangeable_rejects_synonyms | 绿=guard_non_interchangeable_rejects_synonyms
//! // TDD-PROBE: guard_collection_path | 变异：Python yfinance 改为返回 Ok | 红=guard_collection_path_rejects_blocked | 绿=guard_collection_path_rejects_blocked
//! // TDD-PROBE: validate_bar | 变异：缺收盘价时静默补 0 而不是报 Missing | 红=validate_bar_rejects_incomplete_or_illegal | 绿=validate_bar_rejects_incomplete_or_illegal
//! // TDD-PROBE: authorize_yahoo | 变异：证据缺失时默认放行 | 红=authorize_yahoo_is_fail_closed | 绿=authorize_yahoo_is_fail_closed
//! // TDD-PROBE: publication_semantics | 变异：可得性证据层改为 Official | 红=publication_semantics_is_date_inferred_not_eligible | 绿=publication_semantics_is_date_inferred_not_eligible
//! // TDD-PROBE: is_formal_pit_eligible | 变异：改为恒返回 true | 红=is_formal_pit_eligible_is_false | 绿=is_formal_pit_eligible_is_false
//! // TDD-PROBE: parse_yahoo_bars | 变异：未知字段改为静默忽略 | 红=parse_yahoo_bars_rejects_unknown_field | 绿=parse_yahoo_bars_rejects_unknown_field
//! // TDD-PROBE: YahooValue::present | 变异：缺失槽返回 Some(0.0) | 红=yahoo_value_present_never_zero_fills | 绿=yahoo_value_present_never_zero_fills
//! // TDD-PROBE: YahooValue::is_missing | 变异：Present 也判为缺失 | 红=yahoo_value_is_missing_matches_variant | 绿=yahoo_value_is_missing_matches_variant
//! // TDD-PROBE: YahooAuthorizationEvidence::new | 变异：构造时丢弃 scope 字段 | 红=authorization_evidence_new_copies_fields | 绿=authorization_evidence_new_copies_fields

use yahoox::{
    authorize_yahoo, guard_collection_path, guard_non_interchangeable, guard_primary_source,
    is_collection_obligation, is_forbidden_primary, is_formal_pit_eligible, parse_yahoo_bars,
    publication_semantics, unit_for_symbol, validate_bar, AvailabilityEvidence, YahooAuthorization,
    YahooAuthorizationEvidence, YahooBar, YahooCollectionPath, YahooError, YahooErrorKind,
    YahooMissingReason, YahooScope, YahooValue, ACTIVE_COLLECTION_OBLIGATIONS, DECISION_ID,
    FORBIDDEN_PRIMARY_SYMBOLS, NON_INTERCHANGEABLE_PAIRS,
};
use yahoox::{Date, Frequency, Period, PitEligibility, TimePrecision, Unit, AUTHORIZED_SCOPE};

/// 合成样本（`_synthetic` 标注见文件本体）。
const BARS: &str = include_str!("fixtures/bars.json");
const FORBIDDEN: &str = include_str!("fixtures/forbidden_primary.json");
const DUPLICATE: &str = include_str!("fixtures/duplicate_identity.json");

/// `YahooError::kind`：8 个变体各归其类，不靠字符串匹配。
#[test]
fn error_kind_maps_each_variant() {
    let cases = [
        (YahooError::Invalid("x".into()), YahooErrorKind::Invalid),
        (YahooError::Missing("x".into()), YahooErrorKind::Missing),
        (
            YahooError::AuthorizationDenied("x".into()),
            YahooErrorKind::AuthorizationDenied,
        ),
        (
            YahooError::RoutedElsewhere("x".into()),
            YahooErrorKind::RoutedElsewhere,
        ),
        (
            YahooError::WriteAuthorityDenied("x".into()),
            YahooErrorKind::WriteAuthorityDenied,
        ),
        (
            YahooError::SemanticallyRejected("x".into()),
            YahooErrorKind::SemanticallyRejected,
        ),
        (
            YahooError::NotApplicable("x".into()),
            YahooErrorKind::NotApplicable,
        ),
        (YahooError::Invariant("x".into()), YahooErrorKind::Invariant),
    ];
    for (error, expected) in cases {
        assert_eq!(error.kind(), expected, "分类不符：{error:?}");
    }
}

/// `YahooError::is_retryable`：本层无网络，仅 `Invariant` 可重试。
#[test]
fn error_is_retryable_only_for_invariant() {
    assert!(YahooError::Invariant("x".into()).is_retryable());
    assert!(!YahooError::RoutedElsewhere("x".into()).is_retryable());
    assert!(!YahooError::AuthorizationDenied("x".into()).is_retryable());
}

/// `Date::new`：月份、日与闰年三重校验。
#[test]
fn date_new_validates_range() {
    assert!(Date::new(2026, 8, 14).is_ok());
    assert!(Date::new(2024, 2, 29).is_ok(), "2024 是闰年");
    assert!(Date::new(2026, 2, 29).is_err(), "2026 不是闰年");
    assert!(Date::new(2026, 13, 1).is_err());
    assert!(Date::new(2026, 4, 31).is_err());
}

/// `Date::parse`：只接受严格的 `YYYY-MM-DD`。
#[test]
fn date_parse_is_strict_iso() {
    assert_eq!(
        Date::parse("2026-08-14").expect("合法日期"),
        Date {
            year: 2026,
            month: 8,
            day: 14
        }
    );
    for bad in [
        "2026-2-3",
        "2026/02/03",
        "2026-02-03T00:00:00",
        "2026-02-03 ",
    ] {
        assert!(Date::parse(bad).is_err(), "{bad} 必须被拒绝");
    }
}

/// `Period::parse_day`：得到单日期间，而非月 / 年。
#[test]
fn period_parse_day_is_single_day() {
    let period = Period::parse_day("2026-08-14").expect("合法日");
    assert!(matches!(period, Period::Day(_)));
    assert_ne!(period, Period::Year(2026));
}

/// `is_collection_obligation`：义务集恰为 11 符，且与禁抢集不相交。
#[test]
fn collection_obligation_set_is_the_eleven() {
    assert_eq!(ACTIVE_COLLECTION_OBLIGATIONS.len(), 11);
    for symbol in ACTIVE_COLLECTION_OBLIGATIONS {
        assert!(is_collection_obligation(symbol), "{symbol} 应在义务集内");
        assert!(!is_forbidden_primary(symbol), "{symbol} 不得同时是禁抢主源");
    }
    assert!(!is_collection_obligation("^GSPC"));
}

/// `is_forbidden_primary`：禁抢集恰为 9 符。
#[test]
fn forbidden_primary_set_is_the_nine() {
    assert_eq!(FORBIDDEN_PRIMARY_SYMBOLS.len(), 9);
    for symbol in FORBIDDEN_PRIMARY_SYMBOLS {
        assert!(is_forbidden_primary(symbol), "{symbol} 应在禁抢集内");
    }
    assert!(is_forbidden_primary("BTC-USD"), "加密类在禁抢集内");
    assert!(is_forbidden_primary("ETH-USD"), "加密类在禁抢集内");
}

/// `unit_for_symbol`：报价口径按清单品类，且禁抢主源无口径。
#[test]
fn unit_for_symbol_follows_the_manifest() {
    assert_eq!(
        unit_for_symbol("GC=F").expect("义务集符号"),
        Unit::FuturesContinuousQuote
    );
    assert_eq!(
        unit_for_symbol("^STOXX").expect("义务集符号"),
        Unit::IndexPoints
    );
    assert_eq!(
        unit_for_symbol("DX-Y.NYB").expect("义务集符号"),
        Unit::DxyPoints
    );
    assert!(unit_for_symbol("^GSPC").is_err(), "禁抢主源不得有报价口径");
}

/// `guard_primary_source`：禁抢主源 → 路由拒绝；义务集外 → 输入非法。
#[test]
fn guard_primary_source_rejects_usurpers() {
    let routed = guard_primary_source("^GSPC").expect_err("禁抢主源必须拒绝");
    assert_eq!(routed.kind(), YahooErrorKind::RoutedElsewhere);
    let outsider = guard_primary_source("HYG").expect_err("义务集外必须拒绝");
    assert_eq!(outsider.kind(), YahooErrorKind::Invalid);
    assert!(guard_primary_source("GC=F").is_ok());
}

/// `guard_non_interchangeable`：左右互换都必须拒绝。
#[test]
fn guard_non_interchangeable_rejects_synonyms() {
    assert_eq!(NON_INTERCHANGEABLE_PAIRS.len(), 21);
    for pair in NON_INTERCHANGEABLE_PAIRS {
        assert!(guard_non_interchangeable(pair.left, pair.right).is_err());
        assert!(
            guard_non_interchangeable(pair.right, pair.left).is_err(),
            "反向也必须拒绝：{}/{}",
            pair.right,
            pair.left
        );
    }
    assert!(guard_non_interchangeable("TLT", "^RUT").is_ok());
}

/// `guard_collection_path`：`blocked` 路径一律拒绝。
#[test]
fn guard_collection_path_rejects_blocked() {
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

/// `validate_bar`：缺收盘价为 `Missing`（不得转 0），非法排序为 `Invalid`。
#[test]
fn validate_bar_rejects_incomplete_or_illegal() {
    let mut incomplete = sample_bar("GC=F");
    incomplete.close = YahooValue::Missing(YahooMissingReason::SourceOmitted);
    assert_eq!(
        validate_bar(&incomplete).expect_err("缺收盘价").kind(),
        YahooErrorKind::Missing
    );

    let mut inverted = sample_bar("GC=F");
    inverted.high = YahooValue::Present(2400.0);
    inverted.low = YahooValue::Present(2500.0);
    assert_eq!(
        validate_bar(&inverted).expect_err("排序不成立").kind(),
        YahooErrorKind::Invalid
    );

    assert!(validate_bar(&sample_bar("TLT")).is_ok());
}

/// `authorize_yahoo`：证据缺失 / 空白 / live 请求一律拒绝。
#[test]
fn authorize_yahoo_is_fail_closed() {
    let evidence = YahooAuthorizationEvidence::new(DECISION_ID, "ZoneCNH", "domain_yahoo_release");
    match authorize_yahoo(YahooScope::OfflineParseAndTypes, Some(&evidence)) {
        YahooAuthorization::Authorized { scope } => assert!(scope.contains(AUTHORIZED_SCOPE)),
        other => panic!("离线面应授权：{other:?}"),
    }
    assert!(matches!(
        authorize_yahoo(YahooScope::OfflineParseAndTypes, None),
        YahooAuthorization::Denied { .. }
    ));
    let blank = YahooAuthorizationEvidence::new("", "ZoneCNH", "domain");
    assert!(matches!(
        authorize_yahoo(YahooScope::OfflineParseAndTypes, Some(&blank)),
        YahooAuthorization::Denied { .. }
    ));
    assert!(matches!(
        authorize_yahoo(YahooScope::LiveCollection, Some(&evidence)),
        YahooAuthorization::Denied { .. }
    ));
}

/// `publication_semantics`：恒为 `(Date, Inferred, NotEligible)`。
#[test]
fn publication_semantics_is_date_inferred_not_eligible() {
    assert_eq!(
        publication_semantics(),
        (
            TimePrecision::Date,
            AvailabilityEvidence::Inferred,
            PitEligibility::NotEligible
        )
    );
}

/// `is_formal_pit_eligible`：恒为 `false`。
#[test]
fn is_formal_pit_eligible_is_false() {
    assert!(!is_formal_pit_eligible());
}

/// `parse_yahoo_bars`：合成样本可解析；未知字段原子失败。
#[test]
fn parse_yahoo_bars_rejects_unknown_field() {
    let bars = parse_yahoo_bars(BARS).expect("合成样本可解析");
    assert_eq!(bars.len(), 3);
    assert_eq!(
        bars[1].volume,
        YahooValue::Missing(YahooMissingReason::ExplicitNull)
    );

    let unknown = r#"{"bars":[{"symbol":"GC=F","period":"2026-08-14","close":1.0,"x":1}]}"#;
    assert_eq!(
        parse_yahoo_bars(unknown).expect_err("未知字段").kind(),
        YahooErrorKind::SemanticallyRejected
    );
    assert_eq!(
        parse_yahoo_bars(FORBIDDEN).expect_err("禁抢主源").kind(),
        YahooErrorKind::RoutedElsewhere
    );
    assert_eq!(
        parse_yahoo_bars(DUPLICATE).expect_err("重复身份").kind(),
        YahooErrorKind::Invalid
    );
}

/// `YahooValue::present`：缺失不得静默转 0。
#[test]
fn yahoo_value_present_never_zero_fills() {
    assert_eq!(YahooValue::Present(1.5).present(), Some(1.5));
    assert_eq!(
        YahooValue::Missing(YahooMissingReason::SourceOmitted).present(),
        None
    );
    assert_eq!(
        YahooValue::Missing(YahooMissingReason::ExplicitNull).present(),
        None
    );
}

/// `YahooValue::is_missing`：只对缺失变体为真。
#[test]
fn yahoo_value_is_missing_matches_variant() {
    assert!(YahooValue::Missing(YahooMissingReason::SourceOmitted).is_missing());
    assert!(!YahooValue::Present(0.0).is_missing());
}

/// `YahooAuthorizationEvidence::new`：逐字段复制，不丢字段。
#[test]
fn authorization_evidence_new_copies_fields() {
    let evidence = YahooAuthorizationEvidence::new("d-1", "s-1", "scope-1");
    assert_eq!(evidence.decision_id, "d-1");
    assert_eq!(evidence.signed_by, "s-1");
    assert_eq!(evidence.scope, "scope-1");
}

/// 构造一条义务集内的合法日线，供上游用例做定向变异。
fn sample_bar(symbol: &str) -> YahooBar {
    YahooBar {
        symbol: symbol.into(),
        period: Period::Day(Date {
            year: 2026,
            month: 8,
            day: 14,
        }),
        frequency: Frequency::Daily,
        unit: unit_for_symbol(symbol).expect("义务集符号有报价口径"),
        open: YahooValue::Present(100.0),
        high: YahooValue::Present(102.0),
        low: YahooValue::Present(99.0),
        close: YahooValue::Present(101.0),
        volume: YahooValue::Present(1_000.0),
        revision: None,
    }
}
