//! yahoox 的离线解析器：把合成样本 JSON 解析为 [`YahooBar`] 集合。
//!
//! - 参数**只有**字符串：不接受 URL、HTTP 客户端或任何认证信息
//! - 输入形态：单一 JSON 对象，顶层键只允许 `_synthetic` / `_note` / `bars`
//! - 未知字段 → **原子失败**（[`YahooError::SemanticallyRejected`]）
//! - 重复身份（同符号 + 同期间）→ **拒绝**，不做静默去重
//! - `frequency` 与 `unit` 由符号推导，**不由样本自证**（样本不得自带口径声明）

use crate::error::{YahooError, YahooResult};
use crate::value::{
    guard_primary_source, unit_for_symbol, validate_bar, Frequency, Period, YahooBar,
    YahooMissingReason, YahooValue,
};

/// 顶层允许的键。
const TOP_LEVEL_KEYS: [&str; 3] = ["_synthetic", "_note", "bars"];

/// 单条观测允许的键。
const BAR_KEYS: [&str; 7] = ["symbol", "period", "open", "high", "low", "close", "volume"];

/// 解析合成样本 JSON 为日线观测集合。
///
/// # Examples
///
/// ```
/// use yahoox::parse_yahoo_bars;
///
/// let sample = r#"{"bars":[{"symbol":"GC=F","period":"2026-08-14","close":2466.8}]}"#;
/// let bars = parse_yahoo_bars(sample)?;
/// assert_eq!(bars[0].symbol, "GC=F");
/// # Ok::<(), yahoox::YahooError>(())
/// ```
///
/// # Errors
///
/// - 非合法 JSON / 顶层或条目不是对象 / `bars` 不是数组 / 字段类型不符 → [`YahooError::Invalid`]
/// - 缺少 `bars` / 缺少必需字符串字段 / 缺少收盘价 → [`YahooError::Missing`]
/// - 出现未知字段 → [`YahooError::SemanticallyRejected`]
/// - 符号为禁抢主源 → [`YahooError::RoutedElsewhere`]
pub fn parse_yahoo_bars(input: &str) -> YahooResult<Vec<YahooBar>> {
    let root: serde_json::Value = serde_json::from_str(input)
        .map_err(|error| YahooError::Invalid(format!("样本不是合法 JSON：{error}")))?;
    let Some(object) = root.as_object() else {
        return Err(YahooError::Invalid("样本顶层必须是 JSON 对象".into()));
    };
    for key in object.keys() {
        if !TOP_LEVEL_KEYS.contains(&key.as_str()) {
            return Err(YahooError::SemanticallyRejected(format!(
                "顶层未知字段：{key}"
            )));
        }
    }
    let Some(items) = object.get("bars") else {
        return Err(YahooError::Missing("缺少顶层必需项 bars".into()));
    };
    let Some(items) = items.as_array() else {
        return Err(YahooError::Invalid("bars 必须是数组".into()));
    };

    let mut bars: Vec<YahooBar> = Vec::with_capacity(items.len());
    let mut seen: Vec<(String, String)> = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let Some(entry) = item.as_object() else {
            return Err(YahooError::Invalid(format!(
                "bars[{index}] 必须是 JSON 对象"
            )));
        };
        for key in entry.keys() {
            if !BAR_KEYS.contains(&key.as_str()) {
                return Err(YahooError::SemanticallyRejected(format!(
                    "bars[{index}] 未知字段：{key}"
                )));
            }
        }

        let symbol = required_str(entry, index, "symbol")?;
        guard_primary_source(&symbol)?;
        let period_text = required_str(entry, index, "period")?;
        let period = Period::parse_day(&period_text)?;

        let identity = (symbol.clone(), period_text);
        if seen.contains(&identity) {
            return Err(YahooError::Invalid(format!(
                "重复身份：{} @ {}",
                identity.0, identity.1
            )));
        }
        seen.push(identity);

        let unit = unit_for_symbol(&symbol)?;
        let bar = YahooBar {
            symbol,
            period,
            frequency: Frequency::Daily,
            unit,
            open: optional_value(entry, index, "open")?,
            high: optional_value(entry, index, "high")?,
            low: optional_value(entry, index, "low")?,
            close: optional_value(entry, index, "close")?,
            volume: optional_value(entry, index, "volume")?,
            revision: None,
        };
        validate_bar(&bar)?;
        bars.push(bar);
    }
    Ok(bars)
}

/// 取必填字符串字段。
fn required_str(
    entry: &serde_json::Map<String, serde_json::Value>,
    index: usize,
    key: &str,
) -> YahooResult<String> {
    let Some(raw) = entry.get(key) else {
        return Err(YahooError::Missing(format!("bars[{index}] 缺少字段 {key}")));
    };
    let Some(text) = raw.as_str() else {
        return Err(YahooError::Invalid(format!(
            "bars[{index}] 的 {key} 必须是字符串"
        )));
    };
    if text.trim().is_empty() {
        return Err(YahooError::Invalid(format!(
            "bars[{index}] 的 {key} 不得为空白"
        )));
    }
    Ok(text.to_string())
}

/// 取可选数值字段：缺省 → `SourceOmitted`；显式 `null` → `ExplicitNull`。
fn optional_value(
    entry: &serde_json::Map<String, serde_json::Value>,
    index: usize,
    key: &str,
) -> YahooResult<YahooValue> {
    let Some(raw) = entry.get(key) else {
        return Ok(YahooValue::Missing(YahooMissingReason::SourceOmitted));
    };
    if raw.is_null() {
        return Ok(YahooValue::Missing(YahooMissingReason::ExplicitNull));
    }
    let Some(number) = raw.as_f64() else {
        return Err(YahooError::Invalid(format!(
            "bars[{index}] 的 {key} 必须是数值或 null"
        )));
    };
    if !number.is_finite() {
        return Err(YahooError::Invalid(format!(
            "bars[{index}] 的 {key} 必须是有限数值"
        )));
    }
    Ok(YahooValue::Present(number))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{
        "_synthetic": true,
        "_note": "合成样本",
        "bars": [
            {"symbol":"GC=F","period":"2026-08-14","open":2450.5,"high":2471.0,
             "low":2440.2,"close":2466.8,"volume":123456},
            {"symbol":"USDCNH=X","period":"2026-08-14","open":7.11,"high":7.16,
             "low":7.09,"close":7.14,"volume":null}
        ]
    }"#;

    #[test]
    fn parses_a_well_formed_sample() {
        let bars = parse_yahoo_bars(GOOD).expect("合成样本可解析");
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].symbol, "GC=F");
        assert_eq!(bars[0].frequency, Frequency::Daily);
        assert_eq!(
            bars[1].volume,
            YahooValue::Missing(YahooMissingReason::ExplicitNull)
        );
        assert!(bars[0].revision.is_none());
    }

    #[test]
    fn rejects_missing_required_fields() {
        let no_symbol = r#"{"bars":[{"period":"2026-08-14","close":1.0}]}"#;
        assert_eq!(
            parse_yahoo_bars(no_symbol).expect_err("缺 symbol").kind(),
            crate::error::YahooErrorKind::Missing
        );
        let no_period = r#"{"bars":[{"symbol":"GC=F","close":1.0}]}"#;
        assert_eq!(
            parse_yahoo_bars(no_period).expect_err("缺 period").kind(),
            crate::error::YahooErrorKind::Missing
        );
        let no_close = r#"{"bars":[{"symbol":"GC=F","period":"2026-08-14"}]}"#;
        assert_eq!(
            parse_yahoo_bars(no_close).expect_err("缺 close").kind(),
            crate::error::YahooErrorKind::Missing
        );
    }

    #[test]
    fn rejects_unknown_field_atomically() {
        let sample = r#"{"bars":[{"symbol":"GC=F","period":"2026-08-14","close":1.0,"note":"x"}]}"#;
        let error = parse_yahoo_bars(sample).expect_err("未知字段必须原子失败");
        assert_eq!(
            error.kind(),
            crate::error::YahooErrorKind::SemanticallyRejected
        );
        let sample = r#"{"bars":[],"extra":1}"#;
        assert_eq!(
            parse_yahoo_bars(sample).expect_err("顶层未知字段").kind(),
            crate::error::YahooErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn rejects_illegal_date_and_duplicate_identity() {
        let bad_date = r#"{"bars":[{"symbol":"GC=F","period":"2026-8-14","close":1.0}]}"#;
        assert!(parse_yahoo_bars(bad_date).is_err());

        let duplicate = r#"{"bars":[
            {"symbol":"GC=F","period":"2026-08-14","close":1.0},
            {"symbol":"GC=F","period":"2026-08-14","close":2.0}]}"#;
        let error = parse_yahoo_bars(duplicate).expect_err("重复身份必须拒绝");
        assert_eq!(error.kind(), crate::error::YahooErrorKind::Invalid);
    }

    #[test]
    fn rejects_forbidden_primary_and_outsiders() {
        let forbidden = r#"{"bars":[{"symbol":"^GSPC","period":"2026-08-14","close":1.0}]}"#;
        assert_eq!(
            parse_yahoo_bars(forbidden).expect_err("禁抢主源").kind(),
            crate::error::YahooErrorKind::RoutedElsewhere
        );
        let outsider = r#"{"bars":[{"symbol":"HYG","period":"2026-08-14","close":1.0}]}"#;
        assert_eq!(
            parse_yahoo_bars(outsider).expect_err("义务集外").kind(),
            crate::error::YahooErrorKind::Invalid
        );
    }

    #[test]
    fn rejects_non_object_root_and_missing_bars() {
        assert!(parse_yahoo_bars("[]").is_err());
        assert!(parse_yahoo_bars("not json").is_err());
        assert_eq!(
            parse_yahoo_bars("{}").expect_err("缺 bars").kind(),
            crate::error::YahooErrorKind::Missing
        );
    }
}
