//! yahoox 的日线观测值对象与完整性校验入口。

use crate::error::{YahooError, YahooResult};
use crate::value::symbols::{guard_primary_source, unit_for_symbol};
use crate::value::{Frequency, Period, Unit};

/// 具名缺失原因。
///
/// 缺失 MUST 具名表达，MUST NOT 静默转 0（`source-library-contract.md` §2.1）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YahooMissingReason {
    /// 该字段在源样本中缺省。
    SourceOmitted,
    /// 该字段在源样本中显式为 `null`。
    ExplicitNull,
}

/// 日线中的单个数值槽。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum YahooValue {
    /// 有值。
    Present(f64),
    /// 缺失及其具名原因。
    Missing(YahooMissingReason),
}

impl YahooValue {
    /// 取值；缺失返回 `None`（MUST NOT 转 0）。
    #[must_use]
    pub fn present(&self) -> Option<f64> {
        match self {
            Self::Present(value) => Some(*value),
            Self::Missing(_) => None,
        }
    }

    /// 是否为缺失。
    #[must_use]
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing(_))
    }
}

/// 一条 Yahoo 日线观测（源事实，不含任何派生指标）。
#[derive(Debug, Clone, PartialEq)]
pub struct YahooBar {
    /// 源侧符号（清单的符号形态，如 `GC=F` / `USDCNH=X`）。
    pub symbol: String,
    /// 业务期间（日线固定为 [`Period::Day`]）。
    pub period: Period,
    /// 源侧频率（日线为 [`Frequency::Daily`]）。
    pub frequency: Frequency,
    /// 源侧报价口径（本层不做单位换算）。
    pub unit: Unit,
    /// 开盘价。
    pub open: YahooValue,
    /// 最高价。
    pub high: YahooValue,
    /// 最低价。
    pub low: YahooValue,
    /// 收盘价（日线的必需项）。
    pub close: YahooValue,
    /// 成交量。
    pub volume: YahooValue,
    /// 修订标识：Yahoo 无官方 vintage 面，恒为 `None`，MUST NOT 伪造。
    pub revision: Option<String>,
}

/// 校验一条日线观测的完整性。
///
/// 检查项：主源守卫、报价口径与符号一致、期间为单日、频率为日频、
/// 修订标识未伪造、收盘价存在、数值有限、OHLC 排序自洽。
///
/// # Errors
///
/// - 符号为禁抢主源 → [`YahooError::RoutedElsewhere`]
/// - 符号不在义务集内 / 报价口径不符 / 期间或频率不符 / 修订标识被伪造 /
///   数值非有限 / OHLC 排序不成立 → [`YahooError::Invalid`]
/// - 收盘价缺失 → [`YahooError::Missing`]
pub fn validate_bar(bar: &YahooBar) -> YahooResult<()> {
    guard_primary_source(&bar.symbol)?;

    let expected_unit = unit_for_symbol(&bar.symbol)?;
    if bar.unit != expected_unit {
        return Err(YahooError::Invalid(format!(
            "报价口径与符号不符：{} 应为 {expected_unit:?}，实际 {:?}",
            bar.symbol, bar.unit
        )));
    }

    if !matches!(bar.period, Period::Day(_)) {
        return Err(YahooError::Invalid(format!(
            "日线观测的业务期间必须是单日：{}",
            bar.symbol
        )));
    }

    if let Period::Day(date) = bar.period {
        crate::value::Date::new(date.year, date.month, date.day)?;
    }

    if bar.frequency != Frequency::Daily {
        return Err(YahooError::Invalid(format!(
            "日线观测的频率必须是日频：{}",
            bar.symbol
        )));
    }

    if bar.revision.is_some() {
        return Err(YahooError::Invalid(format!(
            "Yahoo 无官方 vintage 面，修订标识不得伪造：{}",
            bar.symbol
        )));
    }

    if bar.close.is_missing() {
        return Err(YahooError::Missing(format!(
            "日线观测缺少收盘价：{}",
            bar.symbol
        )));
    }

    for (name, slot) in [
        ("open", bar.open),
        ("high", bar.high),
        ("low", bar.low),
        ("close", bar.close),
        ("volume", bar.volume),
    ] {
        if let Some(value) = slot.present() {
            if !value.is_finite() {
                return Err(YahooError::Invalid(format!(
                    "数值必须是有限值：{} 的 {name}",
                    bar.symbol
                )));
            }
        }
    }

    check_ordering(bar)
}

/// OHLC 排序自洽（库内完整性不变量，不外推为源事实）。
fn check_ordering(bar: &YahooBar) -> YahooResult<()> {
    let (Some(high), Some(low)) = (bar.high.present(), bar.low.present()) else {
        return Ok(());
    };
    if high < low {
        return Err(YahooError::Invalid(format!(
            "最高价低于最低价：{}",
            bar.symbol
        )));
    }
    for (name, slot) in [("open", bar.open), ("close", bar.close)] {
        let Some(value) = slot.present() else {
            continue;
        };
        if value < low || value > high {
            return Err(YahooError::Invalid(format!(
                "{name} 不在 [low, high] 区间内：{}",
                bar.symbol
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Date;

    fn bar(symbol: &str) -> YahooBar {
        YahooBar {
            symbol: symbol.into(),
            period: Period::Day(Date {
                year: 2026,
                month: 8,
                day: 14,
            }),
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

    #[test]
    fn validate_accepts_a_well_formed_bar() {
        assert!(validate_bar(&bar("GC=F")).is_ok());
    }

    #[test]
    fn validate_rejects_forbidden_primary_and_outsiders() {
        let mut forbidden = bar("GC=F");
        forbidden.symbol = "^GSPC".into();
        assert_eq!(
            validate_bar(&forbidden).expect_err("禁抢主源").kind(),
            crate::error::YahooErrorKind::RoutedElsewhere
        );

        let mut outsider = bar("GC=F");
        outsider.symbol = "HYG".into();
        assert_eq!(
            validate_bar(&outsider).expect_err("义务集外").kind(),
            crate::error::YahooErrorKind::Invalid
        );
    }

    #[test]
    fn validate_rejects_mismatched_unit_period_frequency_and_revision() {
        let mut wrong_unit = bar("GC=F");
        wrong_unit.unit = Unit::IndexPoints;
        assert!(validate_bar(&wrong_unit).is_err());

        let mut wrong_period = bar("GC=F");
        wrong_period.period = Period::Year(2026);
        assert!(validate_bar(&wrong_period).is_err());

        let mut wrong_frequency = bar("GC=F");
        wrong_frequency.frequency = Frequency::Weekly;
        assert!(validate_bar(&wrong_frequency).is_err());

        let mut fake_revision = bar("GC=F");
        fake_revision.revision = Some("v1".into());
        assert!(validate_bar(&fake_revision).is_err());
    }

    #[test]
    fn validate_treats_missing_close_as_missing_not_zero() {
        let mut missing = bar("GC=F");
        missing.close = YahooValue::Missing(YahooMissingReason::ExplicitNull);
        assert_eq!(
            validate_bar(&missing).expect_err("缺收盘价").kind(),
            crate::error::YahooErrorKind::Missing
        );
        assert_eq!(
            YahooValue::Missing(YahooMissingReason::ExplicitNull).present(),
            None
        );
    }

    #[test]
    fn validate_rejects_non_finite_and_inconsistent_ohlc() {
        let mut nan = bar("GC=F");
        nan.close = YahooValue::Present(f64::NAN);
        assert!(validate_bar(&nan).is_err());

        let mut infinite = bar("GC=F");
        infinite.high = YahooValue::Present(f64::INFINITY);
        assert!(validate_bar(&infinite).is_err());

        let mut inverted = bar("GC=F");
        inverted.high = YahooValue::Present(2400.0);
        inverted.low = YahooValue::Present(2500.0);
        assert!(validate_bar(&inverted).is_err());

        let mut outside = bar("GC=F");
        outside.close = YahooValue::Present(2600.0);
        assert!(validate_bar(&outside).is_err());
    }

    #[test]
    fn validation_rechecks_public_period_dates() {
        for date in [
            Date {
                year: 2026,
                month: 2,
                day: 30,
            },
            Date {
                year: 2026,
                month: 0,
                day: 1,
            },
            Date {
                year: 2026,
                month: 12,
                day: 0,
            },
        ] {
            let mut value = bar("GC=F");
            value.period = Period::Day(date);
            assert!(validate_bar(&value).is_err());
        }
        for period in [
            Period::Month {
                year: 2026,
                month: 2,
            },
            Period::Quarter {
                year: 2026,
                quarter: 1,
            },
            Period::Year(2026),
            Period::Event {
                date: Date::new(2026, 2, 1).unwrap(),
            },
        ] {
            let mut value = bar("GC=F");
            value.period = period;
            assert!(validate_bar(&value).is_err());
        }
        let mut value = bar("GC=F");
        value.period = Period::Day(Date::new(2024, 2, 29).unwrap());
        assert!(validate_bar(&value).is_ok());
    }
}
