# Story 0009 検証記録

Date: 2026-09-02

対象commit: `7a2ccb2 test: specify decided event handoff`、`0e3a3c9 feat: publish the final event decision`、`a8a3141 feat: download one decided calendar event`、`2f332e3 feat: hand off the decided event`、`48d8171 test: expose calendar and share handoff races`、`190e9bb fix: make the decided handoff deterministic`、`af12046 test: pin rejected calendar paths and final labels`、`97faf58 fix: normalize calendar route rejections`

## 結論

Product Story 7の決定結果表示、iCalendar持ち帰り、public URL共有は実装済みである。
決定済みの共有URLは匿名回答formを結果と次の行動へ置き換え、主催者画面も同じcalendar・共有componentを使う。

公開projection、決定後の回答境界、DSTを含むUTC変換、escaping、line folding、raw HTTP、自動test、Clippy、format、Fullstack build、候補版HTTPはPASSした。
実ブラウザーのWeb Share、Clipboard、downloadと実calendar applicationへのimportは未実施のため、Storyはin progressとして残す。

## test-firstの証拠

最初の利用者向けHTML、responsive、public projection、iCalendar、HTTP testは `7a2ccb2` でproduction実装より先にcommitした。
未実装type、field、functionによりcompile errorでREDになった。

独立レビュー後も二つのRED区切りを置いた。

- `48d8171` はUTC `DTSTART`、共有中の二重開始防止、`canShare` 非搭載時のnative share、manual fallbackの実非表示を先に要求した。
- `af12046` はAxum path rejectionの統一404と、決定済み・未決定画面の異なる見出しを先に要求した。

## 公開結果と保存境界

public event projectionには、選択candidate ID、local date、local timeだけを任意のdecisionとして加えた。
event、candidate、decisionは一つのread transactionから読み、joinできないdecisionを部分表示しない。
確定記録時刻、organizer capabilityとhash、回答者名、○△×、ひとことは公開projectionへ含めない。

決定後の新規回答は409にする。
決定前に保存が完了してresponseを失った同一payloadの再試行は、決定後でも冪等な成功へ戻す。
先に成立した回答へ付随する任意のひとことは、決定後も保存できる。

## iCalendar

raw routeは `GET /api/events/{public_id}/calendar.ics` である。
一つの `VCALENDAR` と一つの非反復 `VEVENT` だけを返し、安定UID、生成時UTCの `DTSTAMP`、UTC `DTSTART`、event名、任意の主催者noteを含める。
終了時刻を保存していないため、`DTEND`、`DURATION`、`METHOD`、参加者、alarmを補わない。

保存したlocal startとIANA timezoneはserverで一つのinstantへ解決する。
DST overlapは最初の発生、gapは直前offsetを使い、fileには末尾 `Z` のUTC DATE-TIMEを出す。
独立レビューで、IANA名を持つ固定offset `VTIMEZONE` は実際のtransitionを表さないと判明したため、ADR 0014と実装を改訂した。

TEXTはbackslash、comma、semicolon、改行をescapeし、他の制御文字を拒否する。
foldingはUTF-8文字境界を守り、continuation先頭space込みで各physical lineを75 octets以下にし、CRLFと末尾CRLFを使う。

成功は `text/calendar; charset=utf-8`、ASCII公開IDのattachment filename、`Cache-Control: no-store`、`X-Content-Type-Options: nosniff` を返す。
未決定は409、不正・不在は404、不整合は詳細を伏せた500であり、errorにも `no-store` と `nosniff` を付ける。
不正UTF-8 pathもAxum extractor rejectionを明示的に受け、同じ404へ揃えた。

## 共有UI

calendarは通常の同一origin anchorなので、JavaScriptなしでも取得できる。
共有は常にpublicな `/events/{public_id}` であり、主催者用URLやcapabilityをcomponentへ渡さない。

最初のclick内でWeb Shareを開始し、非対応時だけClipboardへ進む。
`navigator.share` があり `navigator.canShare` がないbrowserでもnative shareを試す。
cancelまたは失敗ではclipboardを自動変更せず、次の明示clickをcopy操作へ変える。
Clipboardも使えない場合はlabel付きreadonly URLを表示し、focusとselectを試す。

操作開始時にsignalを同期的なin-progressへ変え、handler guard、`disabled`、`aria-busy` で連打を止める。
manual copy欄は `hidden` に加えてauthor CSSでも `display: none` を保証する。

## 自動検証

```text
cargo test --all-targets
  PASS: default 64 tests

cargo test --all-targets --no-default-features --features server -- \
  --skip simultaneous_identical_retries_create_one_response \
  --skip simultaneous_identical_comments_create_one_value
cargo test --all-targets --all-features -- \
  --skip simultaneous_identical_retries_create_one_response \
  --skip simultaneous_identical_comments_create_one_value
  PASS: 各131 tests

既知の同時再送2件をserver-onlyとall-featuresで各々単独実行
  PASS: 各構成のlogical 133 testsすべて

cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
dx build --web
git diff --check
  PASS
```

Story 7固有では、handoff UI 6件、iCalendar 6件、calendar HTTP 4件、public decision repository 4件、responsive contractの該当testがPASSした。

## HTTPと稼働状態

- 候補版 `127.0.0.1:8081` と安定版 `127.0.0.1:8082` のrootは同時に200。
- 既存の決定済みeventを候補版から取得し、200、calendar content type、attachment、`no-store`、`nosniff`、一つのVEVENT、UTC `DTSTART` を確認した。
- 不正UUIDは404、percent decode後が不正UTF-8のpathも404で、どちらも `no-store` と `nosniff` を持つ。
- 決定済みpublic pageのSSRは「決定した予定」、決定日時、timezone、calendar link、共有buttonを持ち、匿名回答formを持たない。
- `.ics` の端末download、calendar applicationへのimport、Web Share、Clipboardは実ブラウザーで操作していない。

## 独立レビュー

calendar／server、data／transaction、UI／accessibilityの三つのread-onlyレビューを行った。P0はなかった。

採用して修正した指摘は次である。

- 固定offsetの `VTIMEZONE` が実際のIANA DST transitionを表さないP1。
- `hidden` manual copy欄をauthor CSSの `display: grid` が表示し得るP1。
- 共有中の連打で二つのPromise結果がstateを競合更新し得るP1。
- `canShare` 非搭載browser、決定済み見出し、共有説明のP2。
- 不正UTF-8 pathがhandler前の400となり、安全headerを迂回するP2。

残る指摘と受容範囲は次である。

- `DTSTAMP` は同じUIDでもdownloadごとに更新する。決定時刻を公開せず必須propertyを作る現在のtrade-offであり、clientがrevisionとして扱う可能性をADR 0014へ記録済みである。
- 壊れた保存dataの完全なdomain再検証と、新規回答・決定を直接競合させる専用testはない。通常write経路の検証、foreign key、双方の `BEGIN IMMEDIATE`、順次境界testで補完する。
- `PRODID` は管理domainを持たず、global uniquenessのRFC SHOULDを強く満たさない。公開domainを決めるStoryまで固定値を維持する。
- 500文字の主催者noteを含む320pxの情報階層はcomputed layoutで未検証である。

## 実ブラウザー

次はUNVERIFIEDである。

- 320pxとdesktopのcomputed overflow、最長note、二列から一列へのreflow。
- keyboardとscreen readerによる結果、download、共有、status、manual copyの操作。
- native shareのuser activation、cancel、失敗、連打、Clipboard拒否、focus/select。
- `.ics` download後のApple Calendar、Google Calendar等への実importと表示時刻。

利用可能なin-app browser clientは起動できず、外部Playwright実行の追加承認も得ていない。

## 一次情報

- [RFC 5545](https://www.rfc-editor.org/rfc/rfc5545.html): DATE-TIME、TEXT、content line、VEVENT、VTIMEZONE、UID、DTSTAMP、media typeの根拠。
- [RFC 6266](https://www.rfc-editor.org/rfc/rfc6266.html#section-4.2): attachment responseの根拠。
- [RFC 9111](https://www.rfc-editor.org/rfc/rfc9111.html#section-5.2.2.5): `no-store` の根拠。
- [Dioxus 0.7 Axum Router](https://dioxuslabs.com/learn/0.7/essentials/fullstack/axum/): raw routeとSSR fallbackを同じFullstack routerへ置く根拠。
- [W3C Web Share API](https://www.w3.org/TR/web-share/): user activation、pending share、cancelの根拠。
- [W3C Clipboard API](https://www.w3.org/TR/clipboard-apis/#dom-clipboard-writetext): 明示的copy fallbackの根拠。
- [WHATWG downloading resources](https://html.spec.whatwg.org/multipage/links.html#downloading-resources): native download linkの根拠。

## 証拠の境界

| 層 | 状態 | 証拠 |
| --- | --- | --- |
| Git | PASS | test-first、実装、レビューREDと修正の対象commit |
| Rust test / lint / format | PASS | default 64件、server各logical 133件、Clippy、format |
| Fullstack build | PASS | client、server成果物 |
| local HTTP | PASS | 8081/8082 root、calendar 200、不正UUID／UTF-8 404、安全header、public SSR |
| SQLite | PASS | snapshot、決定projection、決定後回答拒否、保存済みretry |
| Chromium 320px / desktop | UNVERIFIED | browser client起動不良、外部script未承認 |
| native share / clipboard / calendar import | UNVERIFIED | 実端末操作をしていない |
