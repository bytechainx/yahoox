# yahoox

`yahoox` 是 Yahoo 源事实的**离线类型层**：把 `specs/adapter/yahoo.md` 声明的符号与日线形态
表达为稳定 Rust 类型，并提供离线解析、符号守卫与 fail-closed 授权判定。

- 只表达源事实：符号 + OHLCV + 期间 + 报价口径，**不含任何派生指标**
- 零网络：无 HTTP 客户端依赖、无端点字面量、不读凭据、不配代理
- 零内部耦合：不依赖任何 `bytechainx/*` crate，无 `path` 依赖
- 义务集 11 符与禁抢主源 9 符为具名常量；禁抢主源请求一律拒绝
- 禁静默替换：21 条近义非同符号对不得互为别名
- `blocked` 采集路径（Python `yfinance` / Cookie / 代理规避 / 抢 FRED 主源）一律拒绝
- 授权判定 fail-closed：证据缺失 / 空白 / live 请求一律 `Denied`
- publication 语义恒为 `Date` + `Inferred` + `NotEligible`

## 安装

本 crate **不发布到 crates.io**，通过 git 依赖引入：

```toml
[dependencies]
yahoox = { git = "https://github.com/bytechainx/yahoox" }
```

## 最小可运行示例

```rust
use yahoox::{guard_primary_source, parse_yahoo_bars, YahooError};

fn main() -> Result<(), YahooError> {
    let sample = r#"{"bars":[{"symbol":"GC=F","period":"2026-08-14",
        "open":2450.5,"high":2471.0,"low":2440.2,"close":2466.8,"volume":123456}]}"#;
    let bars = parse_yahoo_bars(sample)?;
    assert_eq!(bars[0].symbol, "GC=F");
    assert!(bars[0].revision.is_none(), "不得伪造修订标识");

    // 禁抢主源：^GSPC 已改挂 FRED
    assert!(matches!(
        guard_primary_source("^GSPC"),
        Err(YahooError::RoutedElsewhere(_))
    ));
    Ok(())
}
```

授权判定（fail-closed）：

```rust
use yahoox::{authorize_yahoo, YahooAuthorization, YahooAuthorizationEvidence, YahooScope};

let verdict = authorize_yahoo(YahooScope::OfflineParseAndTypes, None);
assert!(matches!(verdict, YahooAuthorization::Denied { .. }));

let evidence = YahooAuthorizationEvidence::new("YAHOO-PROD-2026-08-17-approve", "ZoneCNH", "domain");
assert!(matches!(
    authorize_yahoo(YahooScope::LiveCollection, Some(&evidence)),
    YahooAuthorization::Denied { .. }
));
```

## 主要内容

| 类型 / 常量 | 作用 |
| --- | --- |
| `YahooBar` / `YahooValue` / `YahooMissingReason` | 日线观测与其具名缺失 |
| `Date` / `Period` / `Frequency` / `Unit` | 日期、期间、频率与源侧报价口径（不引入日期库） |
| `ACTIVE_COLLECTION_OBLIGATIONS` / `FORBIDDEN_PRIMARY_SYMBOLS` | 义务集 11 符 / 禁抢主源 9 符 |
| `NON_INTERCHANGEABLE_PAIRS` / `guard_non_interchangeable` | 21 条禁静默替换对及其守卫 |
| `YahooCollectionPath` / `guard_collection_path` | 采集路径类别与 `blocked` 拒绝 |
| `guard_primary_source` / `unit_for_symbol` / `validate_bar` | 主源守卫、口径推导与完整性校验 |
| `YahooAuthorization` / `authorize_yahoo` | fail-closed 授权判定 |
| `parse_yahoo_bars` | 离线合成样本解析（未知字段原子失败、重复身份拒绝） |
| `TimePrecision` / `AvailabilityEvidence` / `PitEligibility` | publication 语义三元组 |

## 生产误用红线

| 禁止 | 原因 |
| --- | --- |
| 把 `authorize_yahoo` 的 `Authorized` 当作生产就绪 | 它只回答「该范围是否被授权访问」 |
| 把 `parse_yahoo_bars` 当作采集器 | 它只解析本地字符串，无网络能力 |
| 把义务集常量当作已落地的采集能力 | 本轮不实现采集（无 `macro-yahoo-collect`） |
| 用 `SPY` / `QQQ` / `EWJ` 顶替指数符号 | 静默替换；守卫会拒绝 |
| 把 `tests/fixtures/` 的数值表述为「实测」 | 它们是合成样本，不是证据 |

## 非目标

- 不做联网采集（`FR-037`：本层零 HTTP 客户端）
- 不做派生指标（净流动性、利差、Credit Impulse、z-score 等归 analytics）
- 不做单位换算（保留源侧报价口径，换算归下游 Normalize）
- 不做凭据管理、代理配置、存储、分发与调度
- 不做 PIT 升格（证据层为推断，禁静默升格）

## 门禁

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo package --no-verify
```

测试全部离线：样本经 `include_str!` 内联，不访问外网，不读环境变量。

`production_decision = NO-GO`（清单 COMPLETE ≠ ship；authorization ≠ Production Ready）。

## 许可

MIT OR Apache-2.0
