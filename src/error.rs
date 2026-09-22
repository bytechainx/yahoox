//! yahoox 的错误分类与错误类型。

/// 错误分类：按「调用方应如何反应」划分。
///
/// 禁止用字符串匹配替代对本枚举的匹配。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YahooErrorKind {
    /// 输入形态或取值非法（调用方应修数据，重试无意义）。
    Invalid,
    /// 缺少必需项。
    Missing,
    /// 授权判定未通过（fail-closed 落点）。
    AuthorizationDenied,
    /// 该产品不属于本域，已被路由规则拒绝。
    RoutedElsewhere,
    /// 越权写入（权威写入归他域）。
    WriteAuthorityDenied,
    /// 结构可解析但语义不被接受（如近义非同 ID）。
    SemanticallyRejected,
    /// 尚未实现的规划能力。
    NotApplicable,
    /// 不变量被破坏（库内 bug 的信号）。
    Invariant,
}

/// yahoox 错误。保留可区分的分类与来源链。
///
/// 错误消息一律为简体中文，且不回显原始样本正文、凭据或整行配置源码。
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum YahooError {
    /// 输入非法。
    #[error("输入非法：{0}")]
    Invalid(String),
    /// 缺少必需项。
    #[error("缺少必需项：{0}")]
    Missing(String),
    /// 授权判定未通过。
    #[error("授权判定未通过：{0}")]
    AuthorizationDenied(String),
    /// 产品不属于本域。
    #[error("不属于本域，已路由他处：{0}")]
    RoutedElsewhere(String),
    /// 越权写入。
    #[error("越权写入被拒绝：{0}")]
    WriteAuthorityDenied(String),
    /// 语义拒绝。
    #[error("语义不被接受：{0}")]
    SemanticallyRejected(String),
    /// 规划能力未实现。
    #[error("规划能力尚未实现：{0}")]
    NotApplicable(String),
    /// 不变量被破坏。
    #[error("不变量被破坏：{0}")]
    Invariant(String),
}

impl YahooError {
    /// 分类，供调用方按「如何反应」分流。
    #[must_use]
    pub fn kind(&self) -> YahooErrorKind {
        match self {
            Self::Invalid(_) => YahooErrorKind::Invalid,
            Self::Missing(_) => YahooErrorKind::Missing,
            Self::AuthorizationDenied(_) => YahooErrorKind::AuthorizationDenied,
            Self::RoutedElsewhere(_) => YahooErrorKind::RoutedElsewhere,
            Self::WriteAuthorityDenied(_) => YahooErrorKind::WriteAuthorityDenied,
            Self::SemanticallyRejected(_) => YahooErrorKind::SemanticallyRejected,
            Self::NotApplicable(_) => YahooErrorKind::NotApplicable,
            Self::Invariant(_) => YahooErrorKind::Invariant,
        }
    }

    /// 是否值得重试。本层无网络，除 `Invariant` 外一律 `false`。
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(self.kind(), YahooErrorKind::Invariant)
    }
}

/// 本 crate 统一结果别名。
pub type YahooResult<T> = Result<T, YahooError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_maps_every_variant_distinctly() {
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
            assert_eq!(error.kind(), expected, "分类不匹配：{error:?}");
        }
    }

    #[test]
    fn only_invariant_is_retryable() {
        assert!(YahooError::Invariant("x".into()).is_retryable());
        for error in [
            YahooError::Invalid("x".into()),
            YahooError::Missing("x".into()),
            YahooError::AuthorizationDenied("x".into()),
            YahooError::RoutedElsewhere("x".into()),
            YahooError::WriteAuthorityDenied("x".into()),
            YahooError::SemanticallyRejected("x".into()),
            YahooError::NotApplicable("x".into()),
        ] {
            assert!(!error.is_retryable(), "本层无网络，不应可重试：{error:?}");
        }
    }

    #[test]
    fn display_is_non_empty_and_keeps_the_reason() {
        let error = YahooError::RoutedElsewhere("^GSPC 主源归 fredx".into());
        let text = error.to_string();
        assert!(!text.is_empty());
        assert!(text.contains("^GSPC 主源归 fredx"), "实际：{text}");
    }

    #[test]
    fn six_categories_are_distinguishable_without_string_matching() {
        // 授权拒绝 / 路由拒绝 / 越权写入 / 语义拒绝四类必须互不相等。
        let kinds = [
            YahooErrorKind::AuthorizationDenied,
            YahooErrorKind::RoutedElsewhere,
            YahooErrorKind::WriteAuthorityDenied,
            YahooErrorKind::SemanticallyRejected,
        ];
        for (i, left) in kinds.iter().enumerate() {
            for right in kinds.iter().skip(i + 1) {
                assert_ne!(left, right, "分类必须互不相等");
            }
        }
    }
}
