# Changelog — yahoox

本文件记录 `yahoox` 的用户可见变更，遵循 [Keep a Changelog](https://keepachangelog.com/)
与 [Semantic Versioning](https://semver.org/)。

## [Unreleased]

## [0.1.0] - 2026-09-22

### 新增

- 首次落地：Yahoo 源事实的离线类型层，采集范围对齐 `specs/adapter/yahoo.md`。
- `YahooBar` / `YahooValue` / `YahooMissingReason`：日线观测（符号 + OHLCV + 期间 + 报价口径），
  缺失以具名原因表达，MUST NOT 静默转 0。
- `Date` / `Period` / `Frequency` / `Unit`：自有日期与期间类型，严格 `YYYY-MM-DD` 解析
  （校验月日与闰年），不引入 `chrono` / `time`。
- `ACTIVE_COLLECTION_OBLIGATIONS`（R3 义务集 11 符）与 `FORBIDDEN_PRIMARY_SYMBOLS`（禁抢主源 9 符）
  具名常量；`guard_primary_source` 对禁抢主源返回 `RoutedElsewhere`、对义务集外符号返回 `Invalid`。
- `NON_INTERCHANGEABLE_PAIRS`（21 条禁静默替换对）与 `guard_non_interchangeable`（左右对称判定）。
- `YahooCollectionPath` 与 `guard_collection_path`：四个 `blocked` 路径一律拒绝，
  R3 放行路径返回 `NotApplicable`（本轮不实现采集）。
- `YahooAuthorization` / `YahooScope` / `authorize_yahoo`：fail-closed 授权判定，
  证据缺失 / 纯空白 / live 请求一律 `Denied`。
- `TimePrecision` / `AvailabilityEvidence` / `PitEligibility` 与 `publication_semantics`：
  恒为 `(Date, Inferred, NotEligible)`；`is_formal_pit_eligible()` 恒为 `false`。
- `parse_yahoo_bars`：离线合成样本解析（字段白名单、未知字段原子失败、重复身份拒绝）。
- 三类测试：`tests/tdd_contracts.rs`（// TDD-PROBE 表）、`tests/sdd_spec.rs`（// SPEC-MAP 与
  `docs/标准.md` 章节 1:1）、`tests/aidd_boundary.rs`（// AIDD 复核表）；夹具为合成样本。

### 说明

本 crate **不实现任何联网采集**：零 HTTP 客户端依赖、零端点字面量、零凭据读取、零代理配置，
也不依赖任何 `bytechainx/*` crate。仓级 `production_decision = NO-GO`。
`tests/fixtures/` 内的全部样本为**合成样本**，不是真实源数据，不构成任何证据。
