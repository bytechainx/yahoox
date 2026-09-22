#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]

//! yahoox —— Yahoo 源事实的离线类型层：日线观测、义务集守卫与 fail-closed 授权判定
//!
//! ## 能力
//!
//! | 能力 | 状态 |
//! | --- | --- |
//! | 日线观测值对象（[`YahooBar`]：符号 + OHLCV + 期间） | 已实现 |
//! | 严格日期 / 期间解析（[`Date`] / [`Period`]，不引入日期库） | 已实现 |
//! | 义务集 11 符与禁抢主源 9 符的具名常量与守卫 | 已实现 |
//! | 禁静默替换对守卫（[`guard_non_interchangeable`]） | 已实现 |
//! | `blocked` 采集路径拒绝（[`guard_collection_path`]） | 已实现 |
//! | 离线合成样本解析（[`parse_yahoo_bars`]） | 已实现 |
//! | fail-closed 授权判定（[`authorize_yahoo`]） | 已实现 |
//! | 联网采集 / PIT 升级 / 派生指标 | **不实现** |
//!
//! ## 责任边界
//!
//! 本库只表达「Yahoo 源侧事实是什么」，并提供离线解析与守卫。
//! 它**不拥有**任何网络能力：无 HTTP 客户端依赖、无端点字面量、不读凭据、不配代理。
//!
//! ## 非目标
//!
//! - 不做联网采集（`macro-yahoo-collect` 未落地；本轮只作声明层常量与守卫）
//! - 不做派生指标（净流动性、利差、z-score 等归 analytics）
//! - 不做单位换算（保留源侧报价口径，换算归下游 Normalize）
//! - 不做存储、分发与调度
//!
//! ## 诚实边界
//!
//! `production_decision = NO-GO`；清单 COMPLETE ≠ ship；authorization ≠ Production Ready。
//! 授权判定只回答「该源的这个范围是否被授权访问」，**不是**「本库是否生产就绪」。
//! `tests/fixtures/` 内的样本全部为**合成样本**，不是真实源数据，不构成任何证据。
//!
//! # 最小示例
//!
//! ```
//! use yahoox::{guard_primary_source, parse_yahoo_bars, YahooError};
//!
//! # let sample = r#"{"bars":[{"symbol":"GC=F","period":"2026-08-14",
//! #     "open":2450.5,"high":2471.0,"low":2440.2,"close":2466.8,"volume":123456}]}"#;
//! let bars = parse_yahoo_bars(sample)?;
//! assert_eq!(bars.len(), 1);
//! assert_eq!(bars[0].symbol, "GC=F");
//!
//! // 禁抢主源：`^GSPC` 已改挂 FRED，不得作为本域主源
//! assert!(matches!(
//!     guard_primary_source("^GSPC"),
//!     Err(YahooError::RoutedElsewhere(_))
//! ));
//! # Ok::<(), YahooError>(())
//! ```

pub mod authz;
pub mod error;
pub mod parse;
pub mod pit;
pub mod value;

pub use authz::{
    authorize_yahoo, YahooAuthorization, YahooAuthorizationEvidence, YahooScope, AUTHORIZED_SCOPE,
    DECISION_ID, NOT_CLAIMS,
};
pub use error::{YahooError, YahooErrorKind, YahooResult};
pub use parse::parse_yahoo_bars;
pub use pit::{
    is_formal_pit_eligible, publication_semantics, AvailabilityEvidence, PitEligibility,
    TimePrecision,
};
pub use value::{
    guard_collection_path, guard_non_interchangeable, guard_primary_source,
    is_collection_obligation, is_forbidden_primary, unit_for_symbol, validate_bar, Date, Frequency,
    NonInterchangeablePair, Period, Unit, YahooBar, YahooCollectionPath, YahooMissingReason,
    YahooValue, ACTIVE_COLLECTION_OBLIGATIONS, FORBIDDEN_PRIMARY_SYMBOLS,
    NON_INTERCHANGEABLE_PAIRS,
};
