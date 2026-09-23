#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! E2E（yahoox）：在**真实夹具文件 + 真实字符串解析 + 真实类型构造与守卫判定**上端到端执行
//! **全部**公开接口。
//!
//! yahoox 是**离线**类型层：零网络、零凭据、零环境变量、零内部耦合，故本用例**不标注为忽略**，
//! 默认参与 CI。
//!
//! 对齐对象是 `cargo +nightly public-api --simplified` 导出的完整公开面（`fn` / `type` /
//! `field` / `const` / `variant` 五类），逐条登记在 [`E2E_MANIFEST`]，运行期由 `cover` 登记表
//! 核对「声明 = 实际执行」（缺一即失败）。
//!
//! **独立核对**：`scripts/verify-e2e-coverage.mjs` 会重新派生公开面与清单**双向** diff，并用
//! `-C instrument-coverage` + `llvm-cov report` 断言每条公开 `fn` 执行次数 > 0；本文件内的
//! 登记表只是**声明**，不是唯一证据。
//!
//! 解析阶段的输入是仓内**真实夹具文件**（`include_str!` 读 `tests/fixtures/*.json`），
//! 外加运行期唯一临时目录的写盘 → 读回 → 解析往返；收尾删除临时目录并断言删除生效。
//!
//! ```text
//! cd /home/workspace/bytechainx/yahoox
//! CARGO_TARGET_DIR=/home/workspace/bytechainx/.cargo/wt/yahoox-e2e-main cargo test --test e2e_yahoo
//! ```

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use yahoox::{
    authorize_yahoo, guard_collection_path, guard_non_interchangeable, guard_primary_source,
    is_collection_obligation, is_forbidden_primary, is_formal_pit_eligible, parse_yahoo_bars,
    publication_semantics, unit_for_symbol, validate_bar, AvailabilityEvidence, Date, Frequency,
    NonInterchangeablePair, Period, PitEligibility, TimePrecision, Unit, YahooAuthorization,
    YahooAuthorizationEvidence, YahooBar, YahooCollectionPath, YahooError, YahooErrorKind,
    YahooMissingReason, YahooResult, YahooScope, YahooValue, ACTIVE_COLLECTION_OBLIGATIONS,
    AUTHORIZED_SCOPE, DECISION_ID, FORBIDDEN_PRIMARY_SYMBOLS, NON_INTERCHANGEABLE_PAIRS,
    NOT_CLAIMS,
};

/// 公开面清单：`(条目类别, 入口 id)`，由 `cargo +nightly public-api --simplified` 派生并冻结。
///
/// 类别取值域：`fn` / `type` / `field` / `const` / `variant`。
/// 该清单是运行时登记的**唯一事实源**——`cover::hit` 拒绝清单外的 id，收尾断言拒绝
/// 「声明了却没执行」的条目。清单本身的时效性由外部核对器与公开面 diff 保证。
///
/// 占位块：由 `node scripts/verify-e2e-coverage.mjs yahoox --write-manifest` 按权威公开面整体重写。
///
/// `#[rustfmt::skip]` 只冻结**排版**：条目的 `(类别, id)` 内容与顺序不变，且核对器的
/// `parseManifest` 依赖「一条一行」形态（其正则不接受 rustfmt 竖排后的尾随逗号）。
#[rustfmt::skip]
const E2E_MANIFEST: &[(&str, &str)] = &[
    ("type", "YahooAuthorization"),
    ("variant", "YahooAuthorization::Authorized"),
    ("variant", "YahooAuthorization::Denied"),
    ("type", "YahooScope"),
    ("variant", "YahooScope::LiveCollection"),
    ("variant", "YahooScope::OfflineParseAndTypes"),
    ("type", "YahooAuthorizationEvidence"),
    ("field", "YahooAuthorizationEvidence::decision_id"),
    ("field", "YahooAuthorizationEvidence::scope"),
    ("field", "YahooAuthorizationEvidence::signed_by"),
    ("fn", "YahooAuthorizationEvidence::new"),
    ("const", "AUTHORIZED_SCOPE"),
    ("const", "DECISION_ID"),
    ("const", "NOT_CLAIMS"),
    ("fn", "authorize_yahoo"),
    ("type", "YahooError"),
    ("variant", "YahooError::AuthorizationDenied"),
    ("variant", "YahooError::Invalid"),
    ("variant", "YahooError::Invariant"),
    ("variant", "YahooError::Missing"),
    ("variant", "YahooError::NotApplicable"),
    ("variant", "YahooError::RoutedElsewhere"),
    ("variant", "YahooError::SemanticallyRejected"),
    ("variant", "YahooError::WriteAuthorityDenied"),
    ("fn", "YahooError::is_retryable"),
    ("fn", "YahooError::kind"),
    ("type", "YahooErrorKind"),
    ("variant", "YahooErrorKind::AuthorizationDenied"),
    ("variant", "YahooErrorKind::Invalid"),
    ("variant", "YahooErrorKind::Invariant"),
    ("variant", "YahooErrorKind::Missing"),
    ("variant", "YahooErrorKind::NotApplicable"),
    ("variant", "YahooErrorKind::RoutedElsewhere"),
    ("variant", "YahooErrorKind::SemanticallyRejected"),
    ("variant", "YahooErrorKind::WriteAuthorityDenied"),
    ("type", "YahooResult"),
    ("fn", "parse_yahoo_bars"),
    ("type", "AvailabilityEvidence"),
    ("variant", "AvailabilityEvidence::Calendar"),
    ("variant", "AvailabilityEvidence::Inferred"),
    ("variant", "AvailabilityEvidence::Official"),
    ("type", "PitEligibility"),
    ("variant", "PitEligibility::Formal"),
    ("variant", "PitEligibility::NotEligible"),
    ("type", "TimePrecision"),
    ("variant", "TimePrecision::Date"),
    ("variant", "TimePrecision::Instant"),
    ("fn", "is_formal_pit_eligible"),
    ("fn", "publication_semantics"),
    ("type", "Frequency"),
    ("variant", "Frequency::Annual"),
    ("variant", "Frequency::Daily"),
    ("variant", "Frequency::Event"),
    ("variant", "Frequency::Irregular"),
    ("variant", "Frequency::Monthly"),
    ("variant", "Frequency::Quarterly"),
    ("variant", "Frequency::Weekly"),
    ("type", "Period"),
    ("variant", "Period::Day"),
    ("variant", "Period::Event"),
    ("variant", "Period::Month"),
    ("variant", "Period::Quarter"),
    ("variant", "Period::Year"),
    ("fn", "Period::parse_day"),
    ("type", "Unit"),
    ("variant", "Unit::DxyPoints"),
    ("variant", "Unit::EtfShareQuote"),
    ("variant", "Unit::FuturesContinuousQuote"),
    ("variant", "Unit::IndexPoints"),
    ("variant", "Unit::OffshoreCnyRate"),
    ("type", "YahooCollectionPath"),
    ("variant", "YahooCollectionPath::CookieCrumbSelfBuilt"),
    ("variant", "YahooCollectionPath::FredPrimaryUsurpation"),
    ("variant", "YahooCollectionPath::ProxyRotationOrChallengeBypass"),
    ("variant", "YahooCollectionPath::PythonYfinance"),
    ("variant", "YahooCollectionPath::YfinanceRsPublicApi"),
    ("type", "YahooMissingReason"),
    ("variant", "YahooMissingReason::ExplicitNull"),
    ("variant", "YahooMissingReason::SourceOmitted"),
    ("type", "YahooValue"),
    ("variant", "YahooValue::Missing"),
    ("variant", "YahooValue::Present"),
    ("fn", "YahooValue::is_missing"),
    ("fn", "YahooValue::present"),
    ("type", "Date"),
    ("field", "Date::day"),
    ("field", "Date::month"),
    ("field", "Date::year"),
    ("fn", "Date::new"),
    ("fn", "Date::parse"),
    ("type", "NonInterchangeablePair"),
    ("field", "NonInterchangeablePair::difference"),
    ("field", "NonInterchangeablePair::left"),
    ("field", "NonInterchangeablePair::right"),
    ("type", "YahooBar"),
    ("field", "YahooBar::close"),
    ("field", "YahooBar::frequency"),
    ("field", "YahooBar::high"),
    ("field", "YahooBar::low"),
    ("field", "YahooBar::open"),
    ("field", "YahooBar::period"),
    ("field", "YahooBar::revision"),
    ("field", "YahooBar::symbol"),
    ("field", "YahooBar::unit"),
    ("field", "YahooBar::volume"),
    ("const", "ACTIVE_COLLECTION_OBLIGATIONS"),
    ("const", "FORBIDDEN_PRIMARY_SYMBOLS"),
    ("const", "NON_INTERCHANGEABLE_PAIRS"),
    ("fn", "guard_collection_path"),
    ("fn", "guard_non_interchangeable"),
    ("fn", "guard_primary_source"),
    ("fn", "is_collection_obligation"),
    ("fn", "is_forbidden_primary"),
    ("fn", "unit_for_symbol"),
    ("fn", "validate_bar"),
];

/// 覆盖登记表：只登记**真实发生**的调用/读取，不登记「计划要调用」。
mod cover {
    use std::collections::BTreeSet;
    use std::sync::{Mutex, OnceLock};

    static EXECUTED: OnceLock<Mutex<BTreeSet<(&'static str, &'static str)>>> = OnceLock::new();

    fn log() -> &'static Mutex<BTreeSet<(&'static str, &'static str)>> {
        EXECUTED.get_or_init(|| Mutex::new(BTreeSet::new()))
    }

    /// 登记一次真实执行。清单外的 `(类别, id)` 立即 panic，防止调用点与清单漂移。
    pub fn hit(kind: &'static str, id: &'static str) {
        assert!(
            super::E2E_MANIFEST
                .iter()
                .any(|(declared_kind, declared_id)| *declared_kind == kind && *declared_id == id),
            "登记了清单外的公开条目：{kind} {id}"
        );
        log().lock().expect("覆盖登记表锁中毒").insert((kind, id));
    }

    pub fn executed() -> BTreeSet<(&'static str, &'static str)> {
        log().lock().expect("覆盖登记表锁中毒").clone()
    }
}

/// 覆盖登记的简写入口（保持调用点可读）。
fn hit(kind: &'static str, id: &'static str) {
    cover::hit(kind, id);
}

/// 清单自身良构：类别取值域合法、`(类别, id)` 不重复、非空。
fn assert_manifest_wellformed() {
    let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
    for (kind, id) in E2E_MANIFEST {
        assert!(
            matches!(*kind, "fn" | "type" | "field" | "const" | "variant"),
            "未知条目类别 {kind}（id={id}）"
        );
        assert!(seen.insert((kind, id)), "清单重复条目：{kind} {id}");
    }
    assert!(!E2E_MANIFEST.is_empty(), "清单不得为空");
}

/// 收尾断言：声明集合与执行集合必须**双向相等**。
fn assert_coverage_complete() {
    let declared: BTreeSet<(&str, &str)> = E2E_MANIFEST.iter().copied().collect();
    let executed = cover::executed();

    let missing: Vec<&(&str, &str)> = declared.difference(&executed).collect();
    let ghost: Vec<&(&str, &str)> = executed.difference(&declared).collect();

    assert!(
        missing.is_empty(),
        "以下 {} 条公开条目被声明却未执行：{missing:?}",
        missing.len()
    );
    assert!(
        ghost.is_empty(),
        "以下 {} 条执行未登记在清单：{ghost:?}",
        ghost.len()
    );
    eprintln!(
        "E2E 覆盖：{}/{} 条公开条目全部执行（yahoox）",
        executed.len(),
        declared.len()
    );
}

/// 进程内唯一的资源名：`<前缀>_<pid>_<纳秒>`。
fn unique_name(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时钟应晚于 UNIX_EPOCH")
        .as_nanos();
    format!("{prefix}_{}_{}", std::process::id(), nanos)
}

/// 临时目录（收尾删除并断言删除生效）。
fn unique_temp_dir(prefix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(unique_name(prefix));
    std::fs::create_dir_all(&dir).expect("创建临时目录必须成功");
    dir
}

/// 构造一条合法日线观测（供校验与解析阶段复用）。
fn make_bar(symbol: &str) -> YahooBar {
    YahooBar {
        symbol: symbol.to_owned(),
        period: Period::Day(Date::new(2026, 8, 14).expect("2026-08-14 合法")),
        frequency: Frequency::Daily,
        unit: unit_for_symbol(symbol).expect("义务集符号有报价口径"),
        open: YahooValue::Present(2450.5),
        high: YahooValue::Present(2471.0),
        low: YahooValue::Present(2440.2),
        close: YahooValue::Present(2466.8),
        volume: YahooValue::Present(123_456.0),
        revision: None,
    }
}

/// 断言授权判定落到 `Denied` 并取出理由。
fn expect_denied(verdict: YahooAuthorization) -> String {
    match verdict {
        YahooAuthorization::Denied { reason } => reason,
        other => panic!("应为 Denied，实得 {other:?}"),
    }
}

/// 阶段 1：6 个公开常量逐条取值断言。
fn phase_constants() {
    // 义务集 11 符：顺序即契约，逐字断言。
    hit("const", "ACTIVE_COLLECTION_OBLIGATIONS");
    assert_eq!(ACTIVE_COLLECTION_OBLIGATIONS.len(), 11);
    assert_eq!(
        ACTIVE_COLLECTION_OBLIGATIONS,
        [
            "GC=F", "HG=F", "CL=F", "RSP", "TLT", "FEZ", "^STOXX", "IWM", "^RUT", "DX-Y.NYB",
            "USDCNH=X"
        ]
    );

    // 禁抢主源 9 符：R1/R2 已改挂 FRED，本域 MUST NOT 争抢。
    hit("const", "FORBIDDEN_PRIMARY_SYMBOLS");
    assert_eq!(FORBIDDEN_PRIMARY_SYMBOLS.len(), 9);
    assert_eq!(
        FORBIDDEN_PRIMARY_SYMBOLS,
        ["^GSPC", "^VIX", "^IXIC", "^N225", "JPY=X", "USDJPY=X", "^TNX", "BTC-USD", "ETH-USD"]
    );

    // 禁静默替换对 21 行：每行左右语义不同且给出差异描述。
    hit("const", "NON_INTERCHANGEABLE_PAIRS");
    assert_eq!(NON_INTERCHANGEABLE_PAIRS.len(), 21);
    for pair in NON_INTERCHANGEABLE_PAIRS {
        let NonInterchangeablePair {
            left,
            right,
            difference,
        } = pair;
        assert_ne!(left, right, "禁替换对的左右不得相同：{left}");
        assert!(
            !difference.trim().is_empty(),
            "禁替换对必须给出语义差异：{left}/{right}"
        );
    }

    hit("const", "DECISION_ID");
    assert_eq!(DECISION_ID, "YAHOO-PROD-2026-08-17-approve");

    hit("const", "AUTHORIZED_SCOPE");
    assert_eq!(AUTHORIZED_SCOPE, "offline_parse_and_types");

    hit("const", "NOT_CLAIMS");
    assert_eq!(NOT_CLAIMS, ["repo_GO", "trading_GO", "package_stable"]);

    // 义务集与禁抢主源集必须不相交（同一符号不得既采集又禁抢）。
    let obligations: BTreeSet<&str> = ACTIVE_COLLECTION_OBLIGATIONS.iter().copied().collect();
    let forbidden: BTreeSet<&str> = FORBIDDEN_PRIMARY_SYMBOLS.iter().copied().collect();
    assert_eq!(obligations.len(), 11, "义务集不得自带重复");
    assert_eq!(forbidden.len(), 9, "禁抢主源集不得自带重复");
    assert!(
        obligations.is_disjoint(&forbidden),
        "义务集与禁抢主源集不得相交"
    );
}

/// 阶段 2：基础值对象（`Date` / `Period` / `Frequency` / `Unit` / `NonInterchangeablePair`）。
fn phase_value_types() {
    // ---- Date：构造 + 严格解析 + 穷尽解构 ----
    hit("type", "Date");
    let valid = Date::new(2026, 8, 14).expect("2026-08-14 合法");
    hit("fn", "Date::new");
    let rejected: [(i16, u8, u8); 5] = [
        (2026, 0, 1),
        (2026, 13, 1),
        (2026, 1, 0),
        (2026, 2, 29),
        (2026, 4, 31),
    ];
    for (year, month, day) in rejected {
        assert!(
            Date::new(year, month, day).is_err(),
            "必须拒绝：{year}-{month}-{day}"
        );
    }
    assert!(Date::new(2024, 2, 29).is_ok(), "2024 是闰年");
    assert!(Date::new(2000, 2, 29).is_ok(), "2000 是闰年");

    let parsed = Date::parse("2026-08-14").expect("严格 ISO 日期");
    hit("fn", "Date::parse");
    for bad in [
        "2026-2-3",
        "2026/02/03",
        "2026-02-03T00:00:00",
        "2026-02-3",
        "2026-002-03",
        "2026-02-030",
        "",
        "2026-02-03 ",
        "2026-02-0a",
    ] {
        assert!(Date::parse(bad).is_err(), "{bad} 必须被拒绝");
    }

    // 穷尽解构（无 `..`）：新增公开字段会在此处编译失败。
    let Date { year, month, day } = parsed;
    hit("field", "Date::year");
    hit("field", "Date::month");
    hit("field", "Date::day");
    assert_eq!((year, month, day), (2026, 8, 14));
    assert_eq!(valid, parsed, "new 与 parse 必须给出同一身份");

    // ---- Frequency：7 变体；变体集完整性由层 1（权威公开面 diff）兜底，不靠本数组 ----
    hit("type", "Frequency");
    let frequencies = [
        Frequency::Daily,
        Frequency::Weekly,
        Frequency::Monthly,
        Frequency::Quarterly,
        Frequency::Annual,
        Frequency::Event,
        Frequency::Irregular,
    ];
    for id in [
        "Frequency::Daily",
        "Frequency::Weekly",
        "Frequency::Monthly",
        "Frequency::Quarterly",
        "Frequency::Annual",
        "Frequency::Event",
        "Frequency::Irregular",
    ] {
        hit("variant", id);
    }
    assert_eq!(frequencies.len(), 7, "频率取值域为 7 个");
    let frequency_names: BTreeSet<String> = frequencies
        .iter()
        .map(|value| format!("{value:?}"))
        .collect();
    assert_eq!(
        frequency_names.len(),
        7,
        "7 个频率变体的 Debug 名必须两两相异"
    );

    // ---- Unit：5 变体 ----
    hit("type", "Unit");
    let units = [
        Unit::IndexPoints,
        Unit::FuturesContinuousQuote,
        Unit::EtfShareQuote,
        Unit::OffshoreCnyRate,
        Unit::DxyPoints,
    ];
    for id in [
        "Unit::IndexPoints",
        "Unit::FuturesContinuousQuote",
        "Unit::EtfShareQuote",
        "Unit::OffshoreCnyRate",
        "Unit::DxyPoints",
    ] {
        hit("variant", id);
    }
    assert_eq!(units.len(), 5, "报价口径取值域为 5 个");
    let unit_names: BTreeSet<String> = units.iter().map(|value| format!("{value:?}")).collect();
    assert_eq!(unit_names.len(), 5, "5 个口径变体的 Debug 名必须两两相异");

    // ---- Period：5 变体 + 单日解析 ----
    hit("type", "Period");
    let day = Date::new(2026, 8, 14).expect("2026-08-14 合法");
    let periods = [
        Period::Day(day),
        Period::Month {
            year: 2026,
            month: 8,
        },
        Period::Quarter {
            year: 2026,
            quarter: 3,
        },
        Period::Year(2026),
        Period::Event { date: day },
    ];
    for id in [
        "Period::Day",
        "Period::Month",
        "Period::Quarter",
        "Period::Year",
        "Period::Event",
    ] {
        hit("variant", id);
    }
    assert_eq!(periods.len(), 5, "期间取值域为 5 个");
    let period_names: BTreeSet<String> = periods.iter().map(|value| format!("{value:?}")).collect();
    assert_eq!(period_names.len(), 5, "5 个期间变体的 Debug 名必须两两相异");

    let parsed_day = Period::parse_day("2026-08-14").expect("合法日");
    hit("fn", "Period::parse_day");
    assert_eq!(parsed_day, Period::Day(day));
    assert!(Period::parse_day("2026-8-14").is_err(), "补零缺失必须拒绝");

    // ---- NonInterchangeablePair：读真实元素 + 穷尽解构（无 `..`）----
    hit("type", "NonInterchangeablePair");
    let sample = NON_INTERCHANGEABLE_PAIRS[0];
    let NonInterchangeablePair {
        left,
        right,
        difference,
    } = sample;
    hit("field", "NonInterchangeablePair::left");
    hit("field", "NonInterchangeablePair::right");
    hit("field", "NonInterchangeablePair::difference");
    assert_ne!(left, right, "禁替换对左右语义必须不同");
    assert!(!left.trim().is_empty() && !right.trim().is_empty());
    assert!(!difference.trim().is_empty(), "差异描述不得为空白");
}

/// 阶段 3：错误面（`YahooError` / `YahooErrorKind` / `YahooResult`）。
fn phase_error_plane() {
    hit("type", "YahooError");
    hit("type", "YahooErrorKind");

    // 一张表：8 个错误变体 × (分类, 是否可重试)。
    let cases: [(&str, YahooError, YahooErrorKind, bool); 8] = [
        (
            "YahooError::Invalid",
            YahooError::Invalid("x".into()),
            YahooErrorKind::Invalid,
            false,
        ),
        (
            "YahooError::Missing",
            YahooError::Missing("x".into()),
            YahooErrorKind::Missing,
            false,
        ),
        (
            "YahooError::AuthorizationDenied",
            YahooError::AuthorizationDenied("x".into()),
            YahooErrorKind::AuthorizationDenied,
            false,
        ),
        (
            "YahooError::RoutedElsewhere",
            YahooError::RoutedElsewhere("x".into()),
            YahooErrorKind::RoutedElsewhere,
            false,
        ),
        (
            "YahooError::WriteAuthorityDenied",
            YahooError::WriteAuthorityDenied("x".into()),
            YahooErrorKind::WriteAuthorityDenied,
            false,
        ),
        (
            "YahooError::SemanticallyRejected",
            YahooError::SemanticallyRejected("x".into()),
            YahooErrorKind::SemanticallyRejected,
            false,
        ),
        (
            "YahooError::NotApplicable",
            YahooError::NotApplicable("x".into()),
            YahooErrorKind::NotApplicable,
            false,
        ),
        (
            "YahooError::Invariant",
            YahooError::Invariant("x".into()),
            YahooErrorKind::Invariant,
            true,
        ),
    ];
    for (id, error, expected_kind, retryable) in &cases {
        hit("variant", id);
        assert_eq!(error.kind(), *expected_kind, "分类不匹配：{id}");
        hit("fn", "YahooError::kind");
        assert_eq!(error.is_retryable(), *retryable, "重试性不匹配：{id}");
        hit("fn", "YahooError::is_retryable");
        assert!(!error.to_string().is_empty(), "Display 不得为空：{id}");
    }
    let error_names: BTreeSet<String> = cases
        .iter()
        .map(|(_, error, _, _)| format!("{error:?}"))
        .collect();
    assert_eq!(error_names.len(), 8, "8 个错误变体的 Debug 名必须两两相异");
    let kind_names: BTreeSet<String> = cases
        .iter()
        .map(|(_, _, kind, _)| format!("{kind:?}"))
        .collect();
    assert_eq!(kind_names.len(), 8, "8 个错误分类必须两两相异");
    // 8 个分类变体均已在表中真实构造；逐个登记并回核其 Debug 名确实出现在表中。
    for id in [
        "YahooErrorKind::Invalid",
        "YahooErrorKind::Missing",
        "YahooErrorKind::AuthorizationDenied",
        "YahooErrorKind::RoutedElsewhere",
        "YahooErrorKind::WriteAuthorityDenied",
        "YahooErrorKind::SemanticallyRejected",
        "YahooErrorKind::NotApplicable",
        "YahooErrorKind::Invariant",
    ] {
        let variant_name = id.trim_start_matches("YahooErrorKind::");
        assert!(kind_names.contains(variant_name), "{id} 未被真实构造");
        hit("variant", id);
    }

    // 仅 Invariant 可重试（本层无网络）。
    let retryable_count = cases
        .iter()
        .filter(|(_, error, _, _)| error.is_retryable())
        .count();
    assert_eq!(retryable_count, 1, "只有 Invariant 可重试");

    // YahooResult：Ok 与 Err 两条路径都真跑。
    hit("type", "YahooResult");
    let ok: YahooResult<u8> = Ok(7);
    match ok {
        Ok(value) => assert_eq!(value, 7, "Ok 路径必须原样透出"),
        Err(error) => panic!("不应失败：{error:?}"),
    }
    let err: YahooResult<u8> = Err(YahooError::Missing("缺项".into()));
    match err {
        Ok(_) => panic!("Err 路径不得成功"),
        Err(error) => assert_eq!(error.kind(), YahooErrorKind::Missing),
    }
}

/// 阶段 4：符号事实与守卫（义务集 / 禁抢主源 / 禁替换对 / 采集路径）。
fn phase_symbols_plane() {
    // 禁抢主源 9 符：一律 RoutedElsewhere。
    for symbol in FORBIDDEN_PRIMARY_SYMBOLS {
        let error = guard_primary_source(symbol).expect_err("禁抢主源必须拒绝");
        hit("fn", "guard_primary_source");
        assert_eq!(error.kind(), YahooErrorKind::RoutedElsewhere, "{symbol}");
    }
    // 义务集 11 符：一律放行。
    for symbol in ACTIVE_COLLECTION_OBLIGATIONS {
        assert!(guard_primary_source(symbol).is_ok(), "{symbol} 应放行");
        hit("fn", "guard_primary_source");
    }
    // 义务集外：fail-closed 落 Invalid。
    let outsider = guard_primary_source("HYG").expect_err("义务集外必须拒绝");
    assert_eq!(outsider.kind(), YahooErrorKind::Invalid);

    // 义务集判定与禁抢集判定的合法集合互为补集关系（在两集合内部）。
    hit("fn", "is_collection_obligation");
    for symbol in ACTIVE_COLLECTION_OBLIGATIONS {
        assert!(is_collection_obligation(symbol), "{symbol}");
        assert!(!is_forbidden_primary(symbol), "{symbol} 不得同时禁抢");
    }
    hit("fn", "is_forbidden_primary");
    for symbol in FORBIDDEN_PRIMARY_SYMBOLS {
        assert!(is_forbidden_primary(symbol), "{symbol}");
        assert!(
            !is_collection_obligation(symbol),
            "{symbol} 不得同时在义务集"
        );
    }
    assert!(!is_collection_obligation("HYG"));
    assert!(!is_forbidden_primary("HYG"));

    // 报价口径映射：必须覆盖全部 11 个义务符号，且逐符取值确定。
    hit("fn", "unit_for_symbol");
    let expected_units: [(&str, Unit); 11] = [
        ("GC=F", Unit::FuturesContinuousQuote),
        ("HG=F", Unit::FuturesContinuousQuote),
        ("CL=F", Unit::FuturesContinuousQuote),
        ("RSP", Unit::EtfShareQuote),
        ("TLT", Unit::EtfShareQuote),
        ("FEZ", Unit::EtfShareQuote),
        ("^STOXX", Unit::IndexPoints),
        ("IWM", Unit::EtfShareQuote),
        ("^RUT", Unit::IndexPoints),
        ("DX-Y.NYB", Unit::DxyPoints),
        ("USDCNH=X", Unit::OffshoreCnyRate),
    ];
    let covered: BTreeSet<&str> = expected_units.iter().map(|(symbol, _)| *symbol).collect();
    let obligations: BTreeSet<&str> = ACTIVE_COLLECTION_OBLIGATIONS.iter().copied().collect();
    assert_eq!(covered, obligations, "口径映射必须恰好覆盖 11 个义务符号");
    for (symbol, expected) in expected_units {
        assert_eq!(
            unit_for_symbol(symbol).expect("义务集符号有报价口径"),
            expected,
            "{symbol} 的口径不符"
        );
    }
    let forbidden_unit = unit_for_symbol("^GSPC").expect_err("禁抢主源无报价口径");
    assert_eq!(forbidden_unit.kind(), YahooErrorKind::RoutedElsewhere);
    let outsider_unit = unit_for_symbol("HYG").expect_err("义务集外无报价口径");
    assert_eq!(outsider_unit.kind(), YahooErrorKind::Invalid);

    // 禁静默替换：21 对，正向与反向都必须拒绝；非同对不得误伤。
    for pair in NON_INTERCHANGEABLE_PAIRS {
        let forward = guard_non_interchangeable(pair.left, pair.right).expect_err("正向必须拒绝");
        hit("fn", "guard_non_interchangeable");
        assert_eq!(
            forward.kind(),
            YahooErrorKind::SemanticallyRejected,
            "{}/{}",
            pair.left,
            pair.right
        );
        let backward = guard_non_interchangeable(pair.right, pair.left).expect_err("反向必须拒绝");
        assert_eq!(
            backward.kind(),
            YahooErrorKind::SemanticallyRejected,
            "{}/{} 反向",
            pair.right,
            pair.left
        );
    }
    assert!(
        guard_non_interchangeable("CL=F", "HG=F").is_ok(),
        "非同对不得误伤"
    );
    assert!(guard_non_interchangeable("GC=F", "GC=F").is_ok());

    // 采集路径：blocked 4 条 → AuthorizationDenied；放行路径 → NotApplicable。
    hit("type", "YahooCollectionPath");
    let paths = [
        YahooCollectionPath::YfinanceRsPublicApi,
        YahooCollectionPath::PythonYfinance,
        YahooCollectionPath::CookieCrumbSelfBuilt,
        YahooCollectionPath::ProxyRotationOrChallengeBypass,
        YahooCollectionPath::FredPrimaryUsurpation,
    ];
    for id in [
        "YahooCollectionPath::YfinanceRsPublicApi",
        "YahooCollectionPath::PythonYfinance",
        "YahooCollectionPath::CookieCrumbSelfBuilt",
        "YahooCollectionPath::ProxyRotationOrChallengeBypass",
        "YahooCollectionPath::FredPrimaryUsurpation",
    ] {
        hit("variant", id);
    }
    assert_eq!(paths.len(), 5, "采集路径取值域为 5 个");
    let path_names: BTreeSet<String> = paths.iter().map(|value| format!("{value:?}")).collect();
    assert_eq!(path_names.len(), 5, "5 条采集路径必须两两相异");

    for path in [
        YahooCollectionPath::PythonYfinance,
        YahooCollectionPath::CookieCrumbSelfBuilt,
        YahooCollectionPath::ProxyRotationOrChallengeBypass,
        YahooCollectionPath::FredPrimaryUsurpation,
    ] {
        let error = guard_collection_path(path).expect_err("blocked 路径必须拒绝");
        hit("fn", "guard_collection_path");
        assert_eq!(
            error.kind(),
            YahooErrorKind::AuthorizationDenied,
            "{path:?}"
        );
    }
    let permitted = guard_collection_path(YahooCollectionPath::YfinanceRsPublicApi)
        .expect_err("本轮不实现采集客户端");
    assert_eq!(permitted.kind(), YahooErrorKind::NotApplicable);
}

/// 阶段 5：日线观测与完整性校验（`YahooBar` / `YahooValue` / `validate_bar`）。
fn phase_validation_plane() {
    // 合法观测：正路径。
    let bar = make_bar("GC=F");
    validate_bar(&bar).expect("合法观测必须通过");
    hit("fn", "validate_bar");

    // 穷尽解构 `YahooBar`（无 `..`）：新增公开字段会在此处编译失败。
    hit("type", "YahooBar");
    let YahooBar {
        symbol,
        period,
        frequency,
        unit,
        open,
        high,
        low,
        close,
        volume,
        revision,
    } = bar.clone();
    hit("field", "YahooBar::symbol");
    hit("field", "YahooBar::period");
    hit("field", "YahooBar::frequency");
    hit("field", "YahooBar::unit");
    hit("field", "YahooBar::open");
    hit("field", "YahooBar::high");
    hit("field", "YahooBar::low");
    hit("field", "YahooBar::close");
    hit("field", "YahooBar::volume");
    hit("field", "YahooBar::revision");
    assert_eq!(symbol, "GC=F");
    assert_eq!(period, Period::Day(Date::new(2026, 8, 14).expect("合法日")));
    assert_eq!(frequency, Frequency::Daily);
    assert_eq!(unit, Unit::FuturesContinuousQuote);
    assert_eq!(open.present(), Some(2450.5));
    assert_eq!(high.present(), Some(2471.0));
    assert_eq!(low.present(), Some(2440.2));
    assert_eq!(close.present(), Some(2466.8));
    assert_eq!(volume.present(), Some(123_456.0));
    assert!(revision.is_none(), "Yahoo 无官方 vintage 面，不得伪造");

    // YahooValue / YahooMissingReason：两侧都真跑。
    hit("type", "YahooValue");
    hit("type", "YahooMissingReason");
    let present = YahooValue::Present(2466.8);
    hit("variant", "YahooValue::Present");
    assert_eq!(present.present(), Some(2466.8));
    hit("fn", "YahooValue::present");
    assert!(!present.is_missing());

    let omitted = YahooValue::Missing(YahooMissingReason::SourceOmitted);
    hit("variant", "YahooValue::Missing");
    hit("variant", "YahooMissingReason::SourceOmitted");
    let null = YahooValue::Missing(YahooMissingReason::ExplicitNull);
    hit("variant", "YahooMissingReason::ExplicitNull");
    assert_ne!(omitted, null, "缺省与显式 null 必须可区分");
    for missing in [omitted, null] {
        assert_eq!(missing.present(), None, "缺失 MUST NOT 静默转 0");
        assert!(missing.is_missing());
        hit("fn", "YahooValue::is_missing");
    }

    // 拒绝路径：每条都落到可区分的分类。
    let mut wrong_unit = make_bar("GC=F");
    wrong_unit.unit = Unit::IndexPoints;
    let error = validate_bar(&wrong_unit).expect_err("口径与符号不符必须拒绝");
    assert_eq!(error.kind(), YahooErrorKind::Invalid);

    let mut wrong_period = make_bar("GC=F");
    wrong_period.period = Period::Year(2026);
    assert!(validate_bar(&wrong_period).is_err(), "期间必须是单日");

    let mut wrong_frequency = make_bar("GC=F");
    wrong_frequency.frequency = Frequency::Weekly;
    assert!(validate_bar(&wrong_frequency).is_err(), "频率必须是日频");

    let mut forged_revision = make_bar("GC=F");
    forged_revision.revision = Some("v1".into());
    assert!(validate_bar(&forged_revision).is_err(), "修订标识不得伪造");

    let mut nan = make_bar("GC=F");
    nan.close = YahooValue::Present(f64::NAN);
    assert!(validate_bar(&nan).is_err(), "NaN 必须拒绝");

    let mut infinite = make_bar("GC=F");
    infinite.high = YahooValue::Present(f64::INFINITY);
    assert!(validate_bar(&infinite).is_err(), "无穷必须拒绝");

    let mut inverted = make_bar("GC=F");
    inverted.high = YahooValue::Present(2400.0);
    inverted.low = YahooValue::Present(2500.0);
    assert!(validate_bar(&inverted).is_err(), "最高价低于最低价必须拒绝");

    let mut outside = make_bar("GC=F");
    outside.close = YahooValue::Present(2600.0);
    assert!(validate_bar(&outside).is_err(), "收盘价越出区间必须拒绝");

    let mut missing_close = make_bar("GC=F");
    missing_close.close = YahooValue::Missing(YahooMissingReason::ExplicitNull);
    let error = validate_bar(&missing_close).expect_err("缺收盘价必须拒绝");
    assert_eq!(
        error.kind(),
        YahooErrorKind::Missing,
        "缺收盘价应落 Missing 而非 Invalid"
    );

    let mut forbidden = make_bar("GC=F");
    forbidden.symbol = "^GSPC".into();
    assert_eq!(
        validate_bar(&forbidden).expect_err("禁抢主源").kind(),
        YahooErrorKind::RoutedElsewhere
    );
}

/// 阶段 6：离线解析（真实夹具 + 拒绝路径）。
fn phase_parse_plane() {
    // 真实夹具：3 条合成观测，含两种具名缺失原因。
    let bars = parse_yahoo_bars(include_str!("fixtures/bars.json")).expect("仓内合成夹具可解析");
    hit("fn", "parse_yahoo_bars");
    assert_eq!(bars.len(), 3);
    assert_eq!(bars[0].symbol, "GC=F");
    assert_eq!(bars[1].symbol, "USDCNH=X");
    assert_eq!(bars[2].symbol, "^RUT");
    for bar in &bars {
        assert_eq!(bar.frequency, Frequency::Daily, "解析出的频率恒为日频");
        assert_eq!(
            bar.period,
            Period::Day(Date::new(2026, 8, 14).expect("合法日"))
        );
        assert!(bar.revision.is_none(), "解析不得伪造修订标识");
    }
    assert_eq!(bars[0].open.present(), Some(2450.5));
    assert_eq!(
        bars[1].volume,
        YahooValue::Missing(YahooMissingReason::ExplicitNull),
        "显式 null 必须记为 ExplicitNull"
    );
    assert_eq!(
        bars[2].volume,
        YahooValue::Missing(YahooMissingReason::SourceOmitted),
        "键缺失必须记为 SourceOmitted"
    );

    // 重复身份：拒绝而非静默去重。
    let duplicate = parse_yahoo_bars(include_str!("fixtures/duplicate_identity.json"))
        .expect_err("重复身份必须拒绝");
    assert_eq!(duplicate.kind(), YahooErrorKind::Invalid);

    // 禁抢主源：夹具整份被路由拒绝。
    let forbidden = parse_yahoo_bars(include_str!("fixtures/forbidden_primary.json"))
        .expect_err("禁抢主源必须拒绝");
    assert_eq!(forbidden.kind(), YahooErrorKind::RoutedElsewhere);

    // 内联拒绝路径：未知字段原子失败、必填缺失、形态非法。
    let rejected: [(&str, &str, YahooErrorKind); 10] = [
        (
            "顶层未知字段",
            r#"{"bars":[],"extra":1}"#,
            YahooErrorKind::SemanticallyRejected,
        ),
        (
            "条目未知字段",
            r#"{"bars":[{"symbol":"GC=F","period":"2026-08-14","close":1.0,"note":"x"}]}"#,
            YahooErrorKind::SemanticallyRejected,
        ),
        (
            "缺 symbol",
            r#"{"bars":[{"period":"2026-08-14","close":1.0}]}"#,
            YahooErrorKind::Missing,
        ),
        (
            "缺 period",
            r#"{"bars":[{"symbol":"GC=F","close":1.0}]}"#,
            YahooErrorKind::Missing,
        ),
        (
            "缺 close",
            r#"{"bars":[{"symbol":"GC=F","period":"2026-08-14"}]}"#,
            YahooErrorKind::Missing,
        ),
        (
            "非法日期",
            r#"{"bars":[{"symbol":"GC=F","period":"2026-8-14","close":1.0}]}"#,
            YahooErrorKind::Invalid,
        ),
        ("bars 非数组", r#"{"bars":{}}"#, YahooErrorKind::Invalid),
        ("根非对象", "[]", YahooErrorKind::Invalid),
        ("非法 JSON", "not json", YahooErrorKind::Invalid),
        ("缺 bars", "{}", YahooErrorKind::Missing),
    ];
    for (label, input, expected_kind) in rejected {
        let error = parse_yahoo_bars(input).expect_err(label);
        assert_eq!(error.kind(), expected_kind, "{label}");
    }
}

/// 阶段 7：fail-closed 授权判定。
fn phase_authz_plane() {
    hit("type", "YahooScope");
    let scopes = [YahooScope::OfflineParseAndTypes, YahooScope::LiveCollection];
    hit("variant", "YahooScope::OfflineParseAndTypes");
    hit("variant", "YahooScope::LiveCollection");
    assert_eq!(scopes.len(), 2, "授权范围取值域为 2 个");
    assert_ne!(scopes[0], scopes[1], "两个范围必须可区分");

    hit("type", "YahooAuthorizationEvidence");
    let evidence = YahooAuthorizationEvidence::new(DECISION_ID, "ZoneCNH", "domain_yahoo_release");
    hit("fn", "YahooAuthorizationEvidence::new");
    // 穷尽解构（无 `..`）：新增公开字段会在此处编译失败。
    let YahooAuthorizationEvidence {
        decision_id,
        signed_by,
        scope,
    } = evidence.clone();
    hit("field", "YahooAuthorizationEvidence::decision_id");
    hit("field", "YahooAuthorizationEvidence::signed_by");
    hit("field", "YahooAuthorizationEvidence::scope");
    assert_eq!(decision_id, DECISION_ID);
    assert_eq!(signed_by, "ZoneCNH");
    assert_eq!(scope, "domain_yahoo_release");

    hit("type", "YahooAuthorization");
    // 证据缺失 → Denied（fail-closed）。
    let denied = authorize_yahoo(YahooScope::OfflineParseAndTypes, None);
    hit("fn", "authorize_yahoo");
    hit("variant", "YahooAuthorization::Denied");
    assert!(!expect_denied(denied).is_empty(), "缺少证据必须拒绝");

    // 三个字段各自空白（含纯空格）→ 一律 Denied。
    for blank in ["", "   "] {
        let blank_id = YahooAuthorizationEvidence::new(blank, "ZoneCNH", "domain_yahoo_release");
        let denied = authorize_yahoo(YahooScope::OfflineParseAndTypes, Some(&blank_id));
        assert!(!expect_denied(denied).is_empty(), "决策 ID 空白必须拒绝");

        let blank_signer =
            YahooAuthorizationEvidence::new(DECISION_ID, blank, "domain_yahoo_release");
        let denied = authorize_yahoo(YahooScope::OfflineParseAndTypes, Some(&blank_signer));
        assert!(!expect_denied(denied).is_empty(), "签署者空白必须拒绝");

        let blank_scope = YahooAuthorizationEvidence::new(DECISION_ID, "ZoneCNH", blank);
        let denied = authorize_yahoo(YahooScope::OfflineParseAndTypes, Some(&blank_scope));
        assert!(!expect_denied(denied).is_empty(), "覆盖范围空白必须拒绝");
    }

    // 联网采集：即使证据有效也一律 Denied。
    let denied = authorize_yahoo(YahooScope::LiveCollection, Some(&evidence));
    assert!(
        !expect_denied(denied).is_empty(),
        "LiveCollection 不得被任何签核覆盖"
    );

    // 有效证据 + 离线面 → Authorized，且 scope 串须含授权范围与决策 ID。
    let verdict = authorize_yahoo(YahooScope::OfflineParseAndTypes, Some(&evidence));
    match verdict {
        YahooAuthorization::Authorized { scope } => {
            hit("variant", "YahooAuthorization::Authorized");
            assert!(scope.contains(AUTHORIZED_SCOPE), "实际：{scope}");
            assert!(scope.contains(DECISION_ID), "实际：{scope}");
        }
        other => panic!("有效证据应授权离线面：{other:?}"),
    }
}

/// 阶段 8：publication 语义三元组。
fn phase_pit_plane() {
    hit("type", "TimePrecision");
    let precisions = [TimePrecision::Date, TimePrecision::Instant];
    hit("variant", "TimePrecision::Date");
    hit("variant", "TimePrecision::Instant");
    assert_eq!(precisions.len(), 2, "时间精度取值域为 2 个");
    assert_ne!(precisions[0], precisions[1]);

    hit("type", "AvailabilityEvidence");
    let evidences = [
        AvailabilityEvidence::Official,
        AvailabilityEvidence::Calendar,
        AvailabilityEvidence::Inferred,
    ];
    hit("variant", "AvailabilityEvidence::Official");
    hit("variant", "AvailabilityEvidence::Calendar");
    hit("variant", "AvailabilityEvidence::Inferred");
    assert_eq!(evidences.len(), 3, "可得性证据层取值域为 3 个");
    let evidence_names: BTreeSet<String> =
        evidences.iter().map(|value| format!("{value:?}")).collect();
    assert_eq!(evidence_names.len(), 3, "3 个证据层必须两两相异");

    hit("type", "PitEligibility");
    let eligibilities = [PitEligibility::Formal, PitEligibility::NotEligible];
    hit("variant", "PitEligibility::Formal");
    hit("variant", "PitEligibility::NotEligible");
    assert_eq!(eligibilities.len(), 2, "PIT 资格取值域为 2 个");
    assert_ne!(eligibilities[0], eligibilities[1]);

    // 三元组穷尽解构（无 `..` 可省）：恒为 (Date, Inferred, NotEligible)。
    let semantics = publication_semantics();
    hit("fn", "publication_semantics");
    let (precision, evidence, eligibility) = semantics;
    assert_eq!(
        (precision, evidence, eligibility),
        (
            TimePrecision::Date,
            AvailabilityEvidence::Inferred,
            PitEligibility::NotEligible
        ),
        "本源 publication 时刻属推断层，不得伪装成 Instant / Official"
    );

    hit("fn", "is_formal_pit_eligible");
    assert!(!is_formal_pit_eligible(), "推断层不得进入正式 PIT");
}

/// 阶段 9：真实临时文件的写盘 → 读回 → 解析往返；返回目录供收尾删除。
fn phase_file_plane() -> PathBuf {
    let dir = unique_temp_dir("yahoox_e2e");

    // 写盘 → 读回：读回文本必须与写入逐字节相同。
    let path = dir.join("bars.json");
    let sample = "{\n  \"_synthetic\": true,\n  \"_note\": \"运行期合成样本\",\n  \"bars\": [\n    {\"symbol\":\"GC=F\",\"period\":\"2026-08-14\",\"open\":2450.5,\"high\":2471.0,\"low\":2440.2,\"close\":2466.8,\"volume\":123456}\n  ]\n}\n";
    std::fs::write(&path, sample).expect("写盘必须成功");
    let read = std::fs::read_to_string(&path).expect("读回必须成功");
    assert_eq!(
        read.as_bytes(),
        sample.as_bytes(),
        "读回文本必须与写入逐字节相同"
    );

    let bars = parse_yahoo_bars(&read).expect("读回的真实文件可解析");
    hit("fn", "parse_yahoo_bars");
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].symbol, "GC=F");
    assert_eq!(bars[0].volume.present(), Some(123_456.0));

    // 真实 I/O 失败路径：不存在的文件必须落 NotFound。
    let error = std::fs::read_to_string(dir.join("absent.json")).expect_err("不存在的文件必须失败");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);

    dir
}

/// 收尾：删除临时目录并断言删除生效。
fn cleanup_files(dir: &Path) {
    std::fs::remove_dir_all(dir).expect("清理临时目录必须成功");
    assert!(!dir.exists(), "清理后临时目录不得残留");
}

/// 单一驱动用例：yahoox 无进程级共享状态，但保持单驱动更易归因。
#[test]
fn e2e_yahoo_all_public_api() {
    assert_manifest_wellformed();
    phase_constants();
    phase_value_types();
    phase_error_plane();
    phase_symbols_plane();
    phase_validation_plane();
    phase_parse_plane();
    phase_authz_plane();
    phase_pit_plane();
    let dir = phase_file_plane();
    cleanup_files(&dir);
    assert_coverage_complete();
}
