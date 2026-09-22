//! yahoox 的授权判定：fail-closed 的只读结论。
//!
//! 判定 MUST NOT 改写清单的原登记值；`Denied` MUST 携带可读理由。
//! 本模块判定的是「该源的这个范围是否被授权访问」，**不是**「本库是否生产就绪」。

/// Owner 签核决策 ID（声明层常量，不是运行时凭据）。
pub const DECISION_ID: &str = "YAHOO-PROD-2026-08-17-approve";

/// 本库的授权范围：仅离线解析与类型。
pub const AUTHORIZED_SCOPE: &str = "offline_parse_and_types";

/// 该签核**未**声称的范围（清单 `not_claims`）。
pub const NOT_CLAIMS: [&str; 3] = ["repo_GO", "trading_GO", "package_stable"];

/// 请求的授权范围。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YahooScope {
    /// 离线解析与类型（本库的唯一授权面）。
    OfflineParseAndTypes,
    /// 联网采集（本库不实现，且未被任何签核覆盖）。
    LiveCollection,
}

/// 授权判定输入：Owner 签核文件中的可核对字段（只读快照）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YahooAuthorizationEvidence {
    /// 决策 ID。
    pub decision_id: String,
    /// 签署者。
    pub signed_by: String,
    /// 该签核覆盖的范围。
    pub scope: String,
}

impl YahooAuthorizationEvidence {
    /// 构造证据快照（不做判定）。
    #[must_use]
    pub fn new(decision_id: &str, signed_by: &str, scope: &str) -> Self {
        Self {
            decision_id: decision_id.into(),
            signed_by: signed_by.into(),
            scope: scope.into(),
        }
    }
}

/// 授权判定结果。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YahooAuthorization {
    /// 证据有效且覆盖本次请求的范围与有效期。
    Authorized {
        /// 被覆盖的源 / 端点 / 模式 / 用途 / 有效期。
        scope: String,
    },
    /// 证据缺失 / 过期 / 签署者不明 / 覆盖范围不明。
    Denied {
        /// 可读的拒绝理由。
        reason: String,
    },
}

/// fail-closed 的授权判定。
///
/// 证据缺失、决策 ID 空白、签署者空白或覆盖范围空白 → **一律** `Denied`；
/// 请求 `LiveCollection` → **一律** `Denied`。MUST NOT 默认放行。
#[must_use]
pub fn authorize_yahoo(
    scope: YahooScope,
    evidence: Option<&YahooAuthorizationEvidence>,
) -> YahooAuthorization {
    if matches!(scope, YahooScope::LiveCollection) {
        return YahooAuthorization::Denied {
            reason: "联网采集未被任何签核覆盖，且本库不实现采集客户端".into(),
        };
    }
    let Some(evidence) = evidence else {
        return YahooAuthorization::Denied {
            reason: "缺少授权证据（fail-closed）".into(),
        };
    };
    if evidence.decision_id.trim().is_empty() {
        return YahooAuthorization::Denied {
            reason: "证据缺少决策 ID（签署者不明）".into(),
        };
    }
    if evidence.signed_by.trim().is_empty() {
        return YahooAuthorization::Denied {
            reason: "证据缺少签署者（签署者不明）".into(),
        };
    }
    if evidence.scope.trim().is_empty() {
        return YahooAuthorization::Denied {
            reason: "证据未声明覆盖范围（覆盖范围不明）".into(),
        };
    }
    YahooAuthorization::Authorized {
        scope: format!(
            "{AUTHORIZED_SCOPE}（decision_id={}，签署者={}）",
            evidence.decision_id, evidence.signed_by
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_evidence_authorizes_only_the_offline_scope() {
        let evidence =
            YahooAuthorizationEvidence::new(DECISION_ID, "ZoneCNH", "domain_yahoo_release");
        let verdict = authorize_yahoo(YahooScope::OfflineParseAndTypes, Some(&evidence));
        match verdict {
            YahooAuthorization::Authorized { scope } => {
                assert!(scope.contains(AUTHORIZED_SCOPE), "实际：{scope}");
                assert!(scope.contains(DECISION_ID));
            }
            other => panic!("应授权离线面：{other:?}"),
        }
    }

    #[test]
    fn missing_evidence_is_denied() {
        let verdict = authorize_yahoo(YahooScope::OfflineParseAndTypes, None);
        assert!(matches!(verdict, YahooAuthorization::Denied { .. }));
    }

    #[test]
    fn unknown_scope_in_evidence_is_denied() {
        for blank in ["", "   "] {
            let evidence =
                YahooAuthorizationEvidence::new(blank, "ZoneCNH", "domain_yahoo_release");
            assert!(
                matches!(
                    authorize_yahoo(YahooScope::OfflineParseAndTypes, Some(&evidence)),
                    YahooAuthorization::Denied { .. }
                ),
                "决策 ID 空白必须拒绝"
            );
            let evidence = YahooAuthorizationEvidence::new(DECISION_ID, blank, "domain");
            assert!(matches!(
                authorize_yahoo(YahooScope::OfflineParseAndTypes, Some(&evidence)),
                YahooAuthorization::Denied { .. }
            ));
            let evidence = YahooAuthorizationEvidence::new(DECISION_ID, "ZoneCNH", blank);
            assert!(matches!(
                authorize_yahoo(YahooScope::OfflineParseAndTypes, Some(&evidence)),
                YahooAuthorization::Denied { .. }
            ));
        }
    }

    #[test]
    fn live_collection_is_denied_even_with_valid_evidence() {
        let evidence =
            YahooAuthorizationEvidence::new(DECISION_ID, "ZoneCNH", "domain_yahoo_release");
        let verdict = authorize_yahoo(YahooScope::LiveCollection, Some(&evidence));
        assert!(matches!(verdict, YahooAuthorization::Denied { .. }));
    }

    #[test]
    fn declaration_constants_match_the_manifest() {
        assert_eq!(DECISION_ID, "YAHOO-PROD-2026-08-17-approve");
        assert_eq!(NOT_CLAIMS, ["repo_GO", "trading_GO", "package_stable"]);
    }
}
