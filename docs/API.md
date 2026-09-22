# yahoox 公开 API

**角色**：Yahoo 源事实的离线类型层（无网络能力）

## 公开消费面

### 值对象

| 类型 / 函数 | 一行语义 |
| --- | --- |
| `Date` | 业务日期（年 / 月 / 日）；`Date::new` 校验月日与闰年，`Date::parse` 严格解析 `YYYY-MM-DD` |
| `Period` | 业务期间枚举（日 / 月 / 季 / 年 / 事件）；`Period::parse_day` 得到单日期间 |
| `Frequency` | 源侧频率（7 值：日 / 周 / 月 / 季 / 年 / 事件 / 不规则） |
| `Unit` | 源侧报价口径（指数点 / 期货连续 / ETF 份额 / 离岸人民币 / DXY），**不做换算** |
| `YahooValue` | 日线数值槽：`Present(f64)` 或 `Missing(YahooMissingReason)`；`present()` 缺失返回 `None` |
| `YahooMissingReason` | 具名缺失原因：`SourceOmitted` / `ExplicitNull` |
| `YahooBar` | 一条日线观测：符号 + 期间 + 频率 + 口径 + OHLCV + 修订标识（恒 `None`） |
| `validate_bar` | 校验日线完整性（主源守卫 / 口径 / 期间 / 频率 / 修订 / 有限性 / OHLC 排序） |

### 符号事实与守卫

| 类型 / 函数 | 一行语义 |
| --- | --- |
| `ACTIVE_COLLECTION_OBLIGATIONS` | R3 义务集 11 符（声明层常量，本轮不实现采集） |
| `FORBIDDEN_PRIMARY_SYMBOLS` | 禁抢主源 9 符（R1/R2 已改挂 FRED） |
| `NonInterchangeablePair` | 一条禁替换对：左符号 / 右符号 / 语义差异 |
| `NON_INTERCHANGEABLE_PAIRS` | 涉及本域的禁静默替换对 21 行 |
| `YahooCollectionPath` | 采集路径类别（1 条 R3 放行 + 4 条 `blocked`） |
| `is_collection_obligation` | 该符号是否属义务集 |
| `is_forbidden_primary` | 该符号是否属禁抢主源集 |
| `unit_for_symbol` | 该符号的源侧报价口径（先经主源守卫） |
| `guard_primary_source` | 禁抢主源 → `RoutedElsewhere`；义务集外 → `Invalid` |
| `guard_non_interchangeable` | 命中禁替换对（含左右互换）→ `SemanticallyRejected` |
| `guard_collection_path` | `blocked` 路径 → `AuthorizationDenied`；R3 放行路径 → `NotApplicable` |

### 授权判定与 publication 语义

| 类型 / 函数 | 一行语义 |
| --- | --- |
| `YahooAuthorization` | 判定结果：`Authorized { scope }` / `Denied { reason }` |
| `YahooScope` | 请求范围：`OfflineParseAndTypes` / `LiveCollection`（后者恒拒） |
| `YahooAuthorizationEvidence` | 证据快照（决策 ID / 签署者 / 覆盖范围），`new` 仅复制不判定 |
| `authorize_yahoo` | fail-closed 判定：证据缺失 / 空白 / live 请求一律 `Denied` |
| `DECISION_ID` / `AUTHORIZED_SCOPE` / `NOT_CLAIMS` | 声明层常量（决策 ID / 本库授权范围 / 未声称范围） |
| `TimePrecision` / `AvailabilityEvidence` / `PitEligibility` | publication 三元组的三轴枚举 |
| `publication_semantics` | 恒返回 `(Date, Inferred, NotEligible)` |
| `is_formal_pit_eligible` | 恒返回 `false` |

### 离线解析

| 函数 | 一行语义 |
| --- | --- |
| `parse_yahoo_bars` | 合成样本 JSON（字符串）→ `Vec<YahooBar>`；未知字段原子失败、重复身份拒绝 |

## 安全语义

- 本库**零 HTTP 客户端依赖、零端点字面量、零凭据读取、零代理配置**。
- 解析入口的参数只有字符串：不接受 URL、客户端或任何认证信息。
- 错误消息为简体中文，不回显原始样本正文、凭据或整行配置源码。
- 缺失值以具名原因表达，**MUST NOT** 静默转 0。
- 禁止静默替换：近义非同符号不得互为别名。

## 最小用法

```rust
use yahoox::{parse_yahoo_bars, YahooError};

# fn demo() -> Result<(), YahooError> {
let sample = r#"{"bars":[{"symbol":"GC=F","period":"2026-08-14",
    "open":2450.5,"high":2471.0,"low":2440.2,"close":2466.8,"volume":123456}]}"#;
let bars = parse_yahoo_bars(sample)?;
assert_eq!(bars[0].symbol, "GC=F");
# Ok(())
# }
```
