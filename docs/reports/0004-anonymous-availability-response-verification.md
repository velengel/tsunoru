# Story 0004 検証記録

Date: 2026-09-02

対象commit: `b7d6a80 feat: answer availability from shared events`、`2349ae5 fix: harden anonymous response boundaries`

## 結論

Product Story 2の実装、入力検証、SQLite保存、再試行の冪等性はローカルの自動testとHTTPで確認した。
共有URLから名前と全候補の○、△、×を送り、回答完了までログインや任意コメントを要求しない。

実ブラウザーを操作する検証は、利用できるbrowser clientの起動不良により未実施である。
したがってStoryの状態はin progressとし、320px表示、keyboard操作、通信失敗後の入力保持をUNVERIFIEDとして残す。

## test-firstの証拠

利用者向けと保存境界のtestは `581f1f1 test: specify anonymous availability response` で実装より先にcommitした。

- `cargo test --test availability_response` は、回答用のdomain型とUIがないcompile errorでREDになった。
- `cargo test --test answer_repository --features server` は、回答aggregateの保存境界がないcompile errorでREDになった。
- responsive testは `.availability-options` がないためREDになった。
- 独立レビュー後は、application HTTP statusを保持する型付きserver function、未知の回答値、request改変、同一再試行、異なるpayloadの競合を先にtestで固定した。

## 自動検証

Story 3まで統合した `d6caa04` で、次を再実行した。

```text
cargo test --all-targets
  PASS: 36 tests

cargo test --all-targets --features server
  PASS: 57 tests

cargo test --all-targets --no-default-features --features server
  PASS: 57 tests

cargo clippy --all-targets --all-features -- -D warnings
  PASS

cargo fmt --check
  PASS

dx build --web
  PASS: client build and server build
```

Story 2固有では、HTMLとdomainの10件、repositoryの11件がPASSした。
一つのtransactionでresponseと全availabilityを保存し、候補の欠落、重複、余分な候補、別eventの候補を拒否する。
同じcapabilityと同じpayloadの再送は一件として成功し、異なるpayloadでは上書きしない。

## HTTPとSQLite

型付きserver functionへ有効な回答を送るとHTTP 200になった。
不正なcapabilityはHTTP 422となり、responseは増えなかった。
未知のavailability文字列はDioxusの型付きrequest境界でhandlerより先に拒否され、保存されなかった。ただしDioxus 0.7はこのdecode失敗をHTTP 500で返す。

SQLiteでは、生の回答capabilityを保存せず、SHA-256 hashだけを64文字で保持することをtestで確認した。
回答と全availabilityのtransaction、同時再試行、foreign key、cascadeも隔離DBで検査した。

候補版8081と検証済み版8082は、2026-09-02の確認時にどちらもHTTP 200だった。

## 実ブラウザー

次はUNVERIFIEDである。

- 320pxとdesktopでの横overflowと見た目。
- pointerを使わない全候補の○、△、×選択と送信。
- validation errorから該当入力へ戻る操作。
- 通信失敗後に名前と選択が残り、そのまま再試行できること。
- 別browser contextとserver再起動をまたぐ保存確認。

利用可能なin-app browser clientは、信頼済みscript pathの解決に失敗して起動できなかった。
外部Playwright scriptの実行は追加権限が必要なため、承認なしには実行していない。

## 証拠の境界

| 層 | 状態 | 証拠 |
| --- | --- | --- |
| Git | PASS | `b7d6a80`、`2349ae5` |
| Rust test / lint / format | PASS | 統合後36件、server込み57件、Clippy、format |
| Fullstack build | PASS | client、server成果物 |
| local HTTP | PASS | 有効回答200、不正capability 422、8081と8082のroot 200 |
| SQLite | PASS | transaction、冪等性、改変拒否、秘密のhash保存 |
| Chromium 320px / desktop | UNVERIFIED | browser client起動不良、外部script未承認 |
| 物理スマートフォン | UNVERIFIED | 実施していない |
