# yahoox 上下文

本文件定义 `yahoox` 与其使用方共享的核心词汇，记录领域含义与能力边界，
不记录具体实现细节或部署决定。

## 角色与边界

**源事实类型层**：把 Yahoo 源侧的符号形态与日线观测收敛成稳定 Rust 类型的一层。
它只表达「源侧事实是什么」，不采集、不换算、不派生。
_Avoid_: 采集器 / 客户端 SDK（本库无任何网络能力）

**义务集**（`ACTIVE_COLLECTION_OBLIGATIONS`，11 符）：R3 政策登记的本域采集义务符号。
**本轮不实现采集**，该常量只作声明与守卫。
_Avoid_: 已采集清单（常量存在 ≠ 有采集能力）

**禁抢主源**（`FORBIDDEN_PRIMARY_SYMBOLS`，9 符）：R1/R2 已改挂 FRED 的符号；
本域 MUST NOT 以其为主源、MUST NOT 争抢。
_Avoid_: 黑名单（它约束的是「主源身份」，不是「能不能引用该符号」）

## 语义身份

**符号 ≠ 别名**：`RSP` 与 `^RUT`、`RSP` 与 `SPY`、`TLT` 与 `^TNX`、`CL=F` 与 `BZ=F`、
`^IXIC` 与 `^NDX`、`^STOXX` 与 `^STOXX50E`、`DX-Y.NYB` 与 `DTWEXBGS` 等**语义不同**，
MUST NOT 互为别名、MUST NOT 静默替换。
_Avoid_: 符号归一（归一只有在清单显式裁决处才允许，本库不做任何归一）

**报价口径**（`Unit`）：Yahoo 日线响应不携带单位字段；本库按清单品类记录口径
（指数点 / 期货连续 / ETF 份额 / 离岸人民币 / DXY），**不做任何换算**。
_Avoid_: 单位（换算归下游 Normalize；本层保留源侧口径）

**具名缺失**（`YahooMissingReason`）：缺省与显式 `null` 是两种不同原因；
缺失 MUST 具名表达，MUST NOT 静默转 0。
_Avoid_: 空值（「空值」不区分原因，无法机械校验）

**重复身份**：同符号 + 同期间视为同一身份；解析器**拒绝**重复，而非静默去重。
_Avoid_: 去重（静默去重会掩盖源侧重复，破坏可审计性）

## 采集路径与授权

**`blocked` 路径**：Python `yfinance`、Cookie / Crumb 自研、代理轮换与挑战规避、
抢 FRED 主源 —— 一律拒绝，且**不是**「暂缓」。
_Avoid_: 受限路径（`blocked` 是政策禁止，不是待批）

**R3 放行路径**：公开 API + 公开 lib（`yfinance-rs`）。它**只说明政策不禁止**，
本库仍不实现采集（`NotApplicable`）。
_Avoid_: 已授权采集（放行路径 ≠ 本库具备采集能力）

**fail-closed 授权判定**：证据缺失 / 决策 ID 空白 / 签署者空白 / 覆盖范围空白 / live 请求
→ **一律** `Denied`。MUST NOT 默认放行。
_Avoid_: 生产就绪判定（本判定回答「该范围是否被授权访问」，
**不是**「本库是否生产就绪」；两者共存不矛盾）

## 时间与证据

**publication 语义**：`TimePrecision::Date` + `AvailabilityEvidence::Inferred` +
`PitEligibility::NotEligible`。fixture 无发布时刻字段，`21:00 UTC` 缺省占位被明确拒绝。
_Avoid_: 发布日期（本层表达的是**身份**与**证据层**，不是某个具体时刻）

**禁止静默升格**：接入官方发布时刻字段或经签批日历前，证据层保持 `Inferred`；
变更须先改清单与契约。
_Avoid_: 缺省时刻（补造 `00:00 UTC` 把 `Date` 伪装成 `Instant` 是违规形态）

**合成样本**：`tests/fixtures/` 内的样本全部为自拟合成样本，不是真实源数据，
不构成任何证据，不得表述为「实测」「核验 PASS」。
_Avoid_: 夹具数据（容易被误读为真实样本）

## 已知缺口

- 本库不含采集能力：`macro-yahoo-collect` 未落地，R3 义务集仅为政策声明。
- 本库不提供官方 vintage 面：`YahooBar::revision` 恒为 `None`，且校验拒绝非 `None` 值。
- `^STOXX` 在 Yahoo 侧的有效性在清单中登记为未核验（`UNKNOWN`），本库不据此做任何断言。

## rust-version 推导

规则：`rust-version` = 依赖图中所有依赖所声明 `rust_version` 的最大值
（`cargo metadata --format-version 1` 的 `rust_version` 字段）。

| 依赖 | 版本 | 其 `rust_version` |
| --- | --- | --- |
| `thiserror` | 2.0.20 | 1.71 |
| `thiserror-impl` | 2.0.20 | 1.71 |
| `serde` | 1.0.229 | 1.56 |
| `serde_core` | 1.0.229 | 1.56 |
| `serde_derive` | 1.0.229 | 1.71 |
| `serde_json` | 1.0.151 | 1.71 |
| `proc-macro2` | 1.0.107 | 1.71 |
| `quote` | 1.0.47 | 1.71 |
| `syn` | 3.0.6 | 1.71 |
| `unicode-ident` | 1.0.26 | 1.71 |
| `itoa` | 1.0.18 | 1.68 |
| `memchr` | 2.8.3 | 1.61 |
| `zmij` | 1.0.23 | 1.71 |

**最大值 = `1.71`**，故本 crate 声明 `rust-version = "1.71"`。
无依赖缺失 `rust_version` 的情形，无需保守取值。
