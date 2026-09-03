# Story 0008 検証記録

Date: 2026-09-02

対象commit: `09b7f35 test: specify explicit event decision`、`4a304aa test: prevent stale decision summary`、`9514b1d test: expose stale refresh ownership`、`025af35 feat: add immutable organizer event decision`

## 結論

Product Story 6の主催者による日程決定は実装済みである。
主催者は回答サマリーと必要に応じた集計表を確認した後、未選択の候補から一つをradioで選び、選択中の日時を読んでから別のsubmitで確定できる。

保存、認可、候補所属、同一再試行、異候補競合、同時request、server再open、summary projection、自動test、Fullstack build、候補版の実HTTPはPASSした。
実ブラウザーで320px、desktop、keyboard、VoiceOverを操作する検証は未実施のため、Storyの状態はin progressとして残す。

## test-firstの証拠

利用者向けHTML、responsive contract、repository、server functionは `09b7f35` で実装より先にcommitした。

- domain、storage、server、UIの未実装symbolにより対象testはcompile errorでREDになった。
- responsive testは `.organizer-decision-section` がないためREDになった。
- summary成功後に古いreadが `decision: null` を上書きする競合を `4a304aa` で先にREDにした。
- 独立レビューで、世代交代したrefreshのpending flagに所有者がない競合を見つけ、`9514b1d` で先にREDにした。

## 保存と競合

`event_decisions` はevent public IDをPRIMARY KEYにし、candidate IDとdatabase生成の `decided_at` を一件だけ保存する。
event削除ではdecisionもcascadeし、決定済みcandidateだけの削除はcomposite foreign keyでrestrictする。
親側にはmigration 0002で追加済みの `(id, event_public_id)` unique indexを再利用し、重複indexを作らない。

repositoryは `BEGIN IMMEDIATE` の中で次の順序を守る。

1. event public IDとorganizer capability hashを照合する。
2. candidateが認可済みeventに属することを確認する。
3. 既存decisionを読む。
4. 同じcandidateなら既存projectionを成功として返す。
5. 異なるcandidateなら409へ変換されるConflictを返し、未決定ならinsertしてcommitする。

回答0件でも保存できる。
二つの異なるcandidateをfile-backed SQLiteへ同時送信すると、一件だけがCreatedになり、もう一件はConflictになった。
同じcandidateの再送はAlreadyDecidedとして同じprojectionを返し、databaseを閉じて開き直した後も同じ結果を復元した。

## private HTTPと秘密情報

endpointは `POST /api/organizer/events/decision` である。
requestの生organizer capabilityはcustom `Debug` で伏せ、serverで64文字の小文字16進数を検証し、SHA-256 hashへ変換して破棄してからrepositoryを呼ぶ。

- 入力形式とcandidate ID不正、認可済みeventに属さないcandidateは422。
- event不在、誤ったcapability、別eventのcapabilityは同じ404。
- 異なる既存decisionは409。
- DB失敗はprivate内容を含まない500。
- 関数へ到達した成功とapplication errorは `Cache-Control: no-store`。

responseはcandidate ID、local date、local timeだけであり、capability、hash、`decided_at`、回答、コメントを含まない。
Dioxus生成wireのdecodeより前には関数内headerを付けられない境界を[ADR 0013](../ADR/0013-keep-typed-dioxus-decode-errors-outside-the-application-contract.md)へ分けて記録した。

## summaryとUI

主催者用summaryは、認可から始まる同じread transactionで任意のdecision projectionを読む。
未確定なら `decision: null`、確定後はcandidate IDと日時を返し、mount時のprivate requestを増やさない。

- DOM順はsummary、必要時だけ開く集計表、日程決定、summary更新である。
- native `fieldset` とradioを使い、候補は作成順を保ち、初期選択とsystem推薦を置かない。
- 選択中の日時と「この日時に確定する」submitを分ける。
- 保存中は二重submitを止める。通常失敗ではsummary、matrix、radio選択を残して再試行できる。
- 成功時はsummary stateのdecisionを更新し、formを変更controlのない確定結果へ置き換える。
- 409ではprivate summaryを再取得し、別tabで先に確定された結果を表示する。
- 320pxでは候補を一列、desktopでは二列とし、44px以上の選択・submit領域とvisible focusを定義した。
- 回答者向け公開decision、共有、iCalendar、calendar account連携は追加していない。

## 非同期requestの所有権

summaryの初期読込、更新、復旧、日程決定は、単調増加するrequest epochで後着結果を捨てる。
日程決定は先行するsummary更新を世代交代させ、古い `decision: null` に戻されない。

独立レビューでは、結果の世代だけを検査しても共有の `refreshing` flagに所有者がなく、停止した旧refreshが復旧後の操作をblockできることが分かった。
refresh専用のowner epochを追加し、復旧または日程決定が先行refreshを明示的にsupersedeするよう修正した。
古いrefreshは、自分がまだownerである場合だけpendingを解除する。

## 自動検証

統合後に次を確認した。

```text
cargo test --all-targets
  PASS: default 57 tests

cargo test --all-targets --no-default-features --features server
cargo test --all-targets --all-features
  PASS: 各112 logical testsを分割実行

cargo clippy --all-targets --all-features -- -D warnings
  PASS

cargo fmt --all -- --check
  PASS

dx build --web
  PASS: client build and server build

git diff --check
  PASS
```

Story 6固有では、UI 6件、repository 7件、server境界5件、responsive contractの該当testがPASSした。

server suiteでは既知の `simultaneous_identical_retries_create_one_response` と `simultaneous_identical_comments_create_one_value` を除く110件を一括実行した。
二件はserver-onlyとall-featuresの各構成で単独実行し、合計112件すべてがPASSした。
単独では各0.01秒で成功するが、一括suite内で停止し得る既存test harnessの問題は解消していない。

## HTTPと稼働状態

- 候補版 `127.0.0.1:8081` と検証済み版 `127.0.0.1:8082` のrootはともに200。
- 候補版でtyped wire shapeの形式不正値は422、存在しないeventは404になり、どちらも `no-store`。
- 実HTTPでeventを新規作成し、最初のcandidate確定は200と最小3fieldのJSONを返した。
- 同じrequestの再送は200で、最初のresponseとbyte単位で同じbodyを返した。
- 別candidateの送信は409と `no-store` になり、responseは生capabilityを含まなかった。
- `input` field自体がないwire JSONはhandler前にDioxusが500を返した。このtransport decode境界はADR 0013の受容範囲である。

実HTTP確認で生成されたorganizer capabilityは `/tmp` のrequest fileだけに置き、値をterminalへ出さず、Git対象へ加えていない。

## 独立レビュー

data、server、UIの担当を入れ替え、三つのread-onlyクロスレビューを行った。
productionのP0はなかった。

一件のP1としてrefresh pending所有権の欠落を採用し、RED testを追加して修正した。
実HTTPを通していないという指摘は、候補版で422、404、200、同一再試行200、409と全application responseの `no-store` を確認して補った。

残る指摘は次である。

- 実行中Dioxus componentの選択保持と応答順反転は、SSRとsource contractが中心で実ブラウザー証拠がない。
- summary snapshotは回答の遅着を直接testするが、decisionの遅着を別fixtureで直接testしていない。
- 同時異候補testは `tokio::join!` とfile-backed WALを使うが、repository内部へbarrierを差し込んで開始点を固定していない。
- migration 0004適用済みのpopulate DBへ0005だけを当てる専用upgrade fixtureはない。

いずれも現行productionの不具合は見つかっておらず、schema、transaction順、再open、実HTTPで補完している。

## 実ブラウザー

次はUNVERIFIEDである。

- 320pxとdesktopでのoverflow、候補一列・二列、長い日時、選択状態、確定結果。
- keyboardだけでradio選択、submit、失敗後再試行、409後の結果確認を操作できること。
- VoiceOverがfieldset、legend、選択中日時、保存中、error、確定結果を読み上げること。
- 回答者contextからprivate decisionを変更できず、URL、DOM、browser storage、consoleへcapabilityが新しく露出しないこと。

利用可能なin-app browser clientは起動できず、外部Playwright scriptは追加承認なしに実行していない。

## 一次情報

- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html): `BEGIN IMMEDIATE` で最初のwrite decisionを直列化する根拠。
- [SQLite: CREATE TABLE](https://www.sqlite.org/lang_createtable.html): PRIMARY KEY、DEFAULT、foreign keyを一tableで定義する根拠。
- [SQLite: Foreign Key Support](https://www.sqlite.org/foreignkeys.html): composite parent keyとCASCADE、RESTRICTの根拠。
- [SQLite: Date And Time Functions](https://www.sqlite.org/lang_datefunc.html): `CURRENT_TIMESTAMP` をUTC記録時刻として扱う根拠。
- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/): Rust引数をJSON bodyへserializeするtyped endpointの根拠。
- [RFC 9110: HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html) と [RFC 9111: HTTP Caching](https://www.rfc-editor.org/rfc/rfc9111.html): 500とPOST responseのcache境界を判断する根拠。

## 証拠の境界

| 層 | 状態 | 証拠 |
| --- | --- | --- |
| Git | PASS | `09b7f35`、`4a304aa`、`9514b1d`、`025af35` |
| Rust test / lint / format | PASS | default 57件、server各112件を分割、Clippy、format |
| Fullstack build | PASS | client、server成果物 |
| local HTTP | PASS | 8081/8082 root 200、422/404/200/retry 200/409、application responseのno-store |
| SQLite | PASS | PK、composite FK、CASCADE/RESTRICT、認可、候補所属、競合、再open |
| Chromium 320px / desktop | UNVERIFIED | browser client起動不良、外部script未承認 |
| VoiceOver / 物理スマートフォン | UNVERIFIED | 実施していない |
