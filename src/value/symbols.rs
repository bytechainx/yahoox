//! yahoox 的符号事实与守卫：义务集、禁抢主源、禁静默替换与 blocked 采集路径。
//!
//! 常量取自 `specs/adapter/yahoo.md` 与 `contracts/cross-source-routing.md` §4–§6，
//! 只作**声明层**使用：本库不实现任何采集客户端。

use crate::error::{YahooError, YahooResult};
use crate::value::Unit;

/// R3 政策的采集义务集（11 符）。
///
/// 出处：`specs/adapter/yahoo.md` §2.8 与 `cross-source-routing.md` §5。
/// **本轮不实现采集**（无 `macro-yahoo-collect`），本常量只作声明与守卫。
pub const ACTIVE_COLLECTION_OBLIGATIONS: [&str; 11] = [
    "GC=F", "HG=F", "CL=F", "RSP", "TLT", "FEZ", "^STOXX", "IWM", "^RUT", "DX-Y.NYB", "USDCNH=X",
];

/// 禁抢主源集（9 符）：R1/R2 已把下列行改挂 FRED，本域 MUST NOT 争抢。
///
/// 出处：`specs/adapter/yahoo.md` §2.8 与 `cross-source-routing.md` §5。
pub const FORBIDDEN_PRIMARY_SYMBOLS: [&str; 9] = [
    "^GSPC", "^VIX", "^IXIC", "^N225", "JPY=X", "USDJPY=X", "^TNX", "BTC-USD", "ETH-USD",
];

/// 一条「近义非同 ID」禁替换对。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonInterchangeablePair {
    /// 左符号。
    pub left: &'static str,
    /// 右符号。
    pub right: &'static str,
    /// 语义差异（逐字取自 `cross-source-routing.md` §4 与清单 §2 的对应行）。
    pub difference: &'static str,
}

/// 涉及本域的禁静默替换对（21 行）。
///
/// 覆盖来源：`cross-source-routing.md` §4 中「涉及库」含 `yahoox` 的各行，
/// 外加清单 §2.1 的 `^GSPC` ≠ `SPY` 行。左右两列语义不同，
/// MUST NOT 互为别名、MUST NOT 静默替换。
pub const NON_INTERCHANGEABLE_PAIRS: [NonInterchangeablePair; 21] = [
    NonInterchangeablePair {
        left: "^GSPC",
        right: "SPY",
        difference: "标普 500 价格指数 ≠ SPY 代理 ETF",
    },
    NonInterchangeablePair {
        left: "SP500",
        right: "SPY",
        difference: "FRED 指数序列 ≠ 代理 ETF",
    },
    NonInterchangeablePair {
        left: "DCOILWTICO",
        right: "CL=F",
        difference: "现货 WTI ≠ 连续期货",
    },
    NonInterchangeablePair {
        left: "PCOPPUSDM",
        right: "HG=F",
        difference: "月频现货 ≠ COMEX 日频期货",
    },
    NonInterchangeablePair {
        left: "NASDAQCOM",
        right: "QQQ",
        difference: "综合指数 ≠ QQQ 代理 ETF",
    },
    NonInterchangeablePair {
        left: "^IXIC",
        right: "QQQ",
        difference: "纳斯达克综合指数 ≠ QQQ 代理 ETF",
    },
    NonInterchangeablePair {
        left: "NIKKEI225",
        right: "EWJ",
        difference: "日经指数序列 ≠ EWJ 代理 ETF",
    },
    NonInterchangeablePair {
        left: "^N225",
        right: "EWJ",
        difference: "日经 225 指数 ≠ EWJ 代理 ETF",
    },
    NonInterchangeablePair {
        left: "DEXCHUS",
        right: "USDCNH",
        difference: "在岸 CNY ≠ 离岸 CNH",
    },
    NonInterchangeablePair {
        left: "DX-Y.NYB",
        right: "DTWEXBGS",
        difference: "ICE 美元指数 ≠ 广义贸易加权美元指数",
    },
    NonInterchangeablePair {
        left: "RSP",
        right: "^RUT",
        difference: "等权标普 500 ETF ≠ 罗素 2000 小盘",
    },
    NonInterchangeablePair {
        left: "RSP",
        right: "SPY",
        difference: "等权 ≠ 市值加权",
    },
    NonInterchangeablePair {
        left: "TLT",
        right: "^TNX",
        difference: "20+ 年国债 ETF ≠ CBOE 10Y 收益率指数",
    },
    NonInterchangeablePair {
        left: "CL=F",
        right: "BZ=F",
        difference: "WTI ≠ Brent",
    },
    NonInterchangeablePair {
        left: "^IXIC",
        right: "^NDX",
        difference: "综合指数 ≠ 纳斯达克 100",
    },
    NonInterchangeablePair {
        left: "^STOXX",
        right: "^STOXX50E",
        difference: "符号形态冲突，^STOXX 有效性未核验",
    },
    NonInterchangeablePair {
        left: "USDCNH",
        right: "USDCNY=X",
        difference: "离岸 CNH ≠ 在岸 CNY（清单符号形态）",
    },
    NonInterchangeablePair {
        left: "USDCNH=X",
        right: "USDCNY=X",
        difference: "离岸 ≠ 在岸（规范形）",
    },
    NonInterchangeablePair {
        left: "JPY=X",
        right: "DEXJPUS",
        difference: "Yahoo 声明层符号 ≠ FRED 独占主源",
    },
    NonInterchangeablePair {
        left: "USDJPY=X",
        right: "DEXJPUS",
        difference: "Yahoo 别名历史 ≠ FRED 独占主源",
    },
    NonInterchangeablePair {
        left: "^TNX",
        right: "DGS10",
        difference: "fixture 样本 ≠ FRED 10Y 名义收益率",
    },
];

/// 采集路径类别。`blocked` 取值取自 `POLICY-collect-free-first-paid-pause.md` §3.4。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YahooCollectionPath {
    /// R3 政策唯一放行路径：公开 API + 公开 lib（`yfinance-rs`）。
    YfinanceRsPublicApi,
    /// Python `yfinance`（`blocked`）。
    PythonYfinance,
    /// Cookie / Crumb 自研（`blocked`）。
    CookieCrumbSelfBuilt,
    /// 代理轮换与挑战规避（`blocked`）。
    ProxyRotationOrChallengeBypass,
    /// 抢 FRED 主源（`blocked`）。
    FredPrimaryUsurpation,
}

/// 该符号是否属于 R3 义务集（11 符）。
#[must_use]
pub fn is_collection_obligation(symbol: &str) -> bool {
    ACTIVE_COLLECTION_OBLIGATIONS.contains(&symbol)
}

/// 该符号是否属于禁抢主源集（9 符）。
#[must_use]
pub fn is_forbidden_primary(symbol: &str) -> bool {
    FORBIDDEN_PRIMARY_SYMBOLS.contains(&symbol)
}

/// 该符号的源侧报价口径。
///
/// # Errors
///
/// 先经 [`guard_primary_source`]：禁抢主源返回 [`YahooError::RoutedElsewhere`]，
/// 不在义务集内返回 [`YahooError::Invalid`]。
pub fn unit_for_symbol(symbol: &str) -> YahooResult<Unit> {
    guard_primary_source(symbol)?;
    Ok(match symbol {
        "^STOXX" | "^RUT" => Unit::IndexPoints,
        "GC=F" | "HG=F" | "CL=F" => Unit::FuturesContinuousQuote,
        "RSP" | "TLT" | "FEZ" | "IWM" => Unit::EtfShareQuote,
        "USDCNH=X" => Unit::OffshoreCnyRate,
        "DX-Y.NYB" => Unit::DxyPoints,
        // guard_primary_source 已保证只可能是上述 11 符之一。
        other => {
            return Err(YahooError::Invariant(format!(
                "义务集常量与报价口径映射不同步：{other}"
            )));
        }
    })
}

/// 守卫：某符号不得作为本域主源。
///
/// 禁抢主源（9 符）→ [`YahooError::RoutedElsewhere`]；
/// 不在义务集（11 符）内 → [`YahooError::Invalid`]（fail-closed，不放行无关符号）。
///
/// # Errors
///
/// 见上。
pub fn guard_primary_source(symbol: &str) -> YahooResult<()> {
    if is_forbidden_primary(symbol) {
        return Err(YahooError::RoutedElsewhere(format!(
            "{symbol} 的主源已改挂 fredx，本域禁抢主源"
        )));
    }
    if !is_collection_obligation(symbol) {
        return Err(YahooError::Invalid(format!(
            "{symbol} 不在 R3 义务集（11 符）内，不得作为本域主源"
        )));
    }
    Ok(())
}

/// 守卫：两个符号不得互为别名、不得静默替换。
///
/// 命中 [`NON_INTERCHANGEABLE_PAIRS`] 任一行（含左右互换）即拒绝。
///
/// # Errors
///
/// 命中禁替换对时返回 [`YahooError::SemanticallyRejected`]。
pub fn guard_non_interchangeable(left: &str, right: &str) -> YahooResult<()> {
    for pair in &NON_INTERCHANGEABLE_PAIRS {
        let forward = pair.left == left && pair.right == right;
        let backward = pair.left == right && pair.right == left;
        if forward || backward {
            return Err(YahooError::SemanticallyRejected(format!(
                "{left} 与 {right} 语义不同（{}），不得互为别名或静默替换",
                pair.difference
            )));
        }
    }
    Ok(())
}

/// 守卫：采集路径必须是 R3 放行路径；`blocked` 路径一律拒绝。
///
/// 即使路径为 R3 放行的 [`YahooCollectionPath::YfinanceRsPublicApi`]，本库也返回
/// [`YahooError::NotApplicable`]——本库不实现任何采集客户端（`FR-037`）。
///
/// # Errors
///
/// `blocked` 路径返回 [`YahooError::AuthorizationDenied`]；
/// 放行路径返回 [`YahooError::NotApplicable`]（本轮不实现采集）。
pub fn guard_collection_path(path: YahooCollectionPath) -> YahooResult<()> {
    match path {
        YahooCollectionPath::YfinanceRsPublicApi => Err(YahooError::NotApplicable(
            "R3 放行的唯一采集路径（公开 API + 公开 lib），但本库不实现采集客户端".into(),
        )),
        YahooCollectionPath::PythonYfinance => Err(YahooError::AuthorizationDenied(
            "POLICY §3.4 blocked：Python yfinance 禁止采集".into(),
        )),
        YahooCollectionPath::CookieCrumbSelfBuilt => Err(YahooError::AuthorizationDenied(
            "POLICY §3.4 blocked：Cookie / Crumb 自研禁止采集".into(),
        )),
        YahooCollectionPath::ProxyRotationOrChallengeBypass => {
            Err(YahooError::AuthorizationDenied(
                "POLICY §3.4 blocked：代理轮换与挑战规避禁止采集".into(),
            ))
        }
        YahooCollectionPath::FredPrimaryUsurpation => Err(YahooError::AuthorizationDenied(
            "POLICY §3.4 blocked：抢 FRED 主源禁止采集".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obligation_and_forbidden_sets_have_the_manifest_sizes() {
        assert_eq!(ACTIVE_COLLECTION_OBLIGATIONS.len(), 11);
        assert_eq!(FORBIDDEN_PRIMARY_SYMBOLS.len(), 9);
        assert_eq!(NON_INTERCHANGEABLE_PAIRS.len(), 21);
        for symbol in ACTIVE_COLLECTION_OBLIGATIONS {
            assert!(is_collection_obligation(symbol), "{symbol} 应在义务集内");
            assert!(!is_forbidden_primary(symbol), "{symbol} 不得在禁抢集内");
        }
        for symbol in FORBIDDEN_PRIMARY_SYMBOLS {
            assert!(is_forbidden_primary(symbol), "{symbol} 应在禁抢集内");
            assert!(!is_collection_obligation(symbol), "{symbol} 不得在义务集内");
        }
    }

    #[test]
    fn primary_source_guard_rejects_usurpers_and_outsiders() {
        for symbol in FORBIDDEN_PRIMARY_SYMBOLS {
            let error = guard_primary_source(symbol).expect_err("禁抢主源必须拒绝");
            assert_eq!(error.kind(), crate::error::YahooErrorKind::RoutedElsewhere);
        }
        // 义务集内全部放行。
        for symbol in ACTIVE_COLLECTION_OBLIGATIONS {
            assert!(guard_primary_source(symbol).is_ok(), "{symbol} 应放行");
        }
        // 义务集之外（如日志规划的扩展符号）fail-closed。
        let outsider = guard_primary_source("HYG").expect_err("义务集外必须拒绝");
        assert_eq!(outsider.kind(), crate::error::YahooErrorKind::Invalid);
    }

    #[test]
    fn non_interchangeable_guard_is_symmetric() {
        for pair in NON_INTERCHANGEABLE_PAIRS {
            assert!(
                guard_non_interchangeable(pair.left, pair.right).is_err(),
                "{}/{} 必须拒绝",
                pair.left,
                pair.right
            );
            assert!(
                guard_non_interchangeable(pair.right, pair.left).is_err(),
                "反向也必须拒绝：{}/{}",
                pair.right,
                pair.left
            );
        }
        assert!(guard_non_interchangeable("GC=F", "GC=F").is_ok());
        assert!(guard_non_interchangeable("CL=F", "HG=F").is_ok());
    }

    #[test]
    fn blocked_paths_are_denied_and_the_permitted_path_is_not_implemented() {
        for path in [
            YahooCollectionPath::PythonYfinance,
            YahooCollectionPath::CookieCrumbSelfBuilt,
            YahooCollectionPath::ProxyRotationOrChallengeBypass,
            YahooCollectionPath::FredPrimaryUsurpation,
        ] {
            let error = guard_collection_path(path).expect_err("blocked 路径必须拒绝");
            assert_eq!(
                error.kind(),
                crate::error::YahooErrorKind::AuthorizationDenied,
                "{path:?}"
            );
        }
        let permitted = guard_collection_path(YahooCollectionPath::YfinanceRsPublicApi)
            .expect_err("本轮不实现采集");
        assert_eq!(
            permitted.kind(),
            crate::error::YahooErrorKind::NotApplicable
        );
    }

    #[test]
    fn unit_mapping_covers_the_obligation_set() {
        assert_eq!(
            unit_for_symbol("GC=F").expect("义务集符号"),
            Unit::FuturesContinuousQuote
        );
        assert_eq!(
            unit_for_symbol("^RUT").expect("义务集符号"),
            Unit::IndexPoints
        );
        assert_eq!(
            unit_for_symbol("TLT").expect("义务集符号"),
            Unit::EtfShareQuote
        );
        assert_eq!(
            unit_for_symbol("USDCNH=X").expect("义务集符号"),
            Unit::OffshoreCnyRate
        );
        assert_eq!(
            unit_for_symbol("DX-Y.NYB").expect("义务集符号"),
            Unit::DxyPoints
        );
        assert!(unit_for_symbol("^GSPC").is_err(), "禁抢主源无报价口径");
    }
}
