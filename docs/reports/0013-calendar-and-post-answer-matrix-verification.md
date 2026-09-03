# Story 0013・0014 検証記録

Date: 2026-09-02

対象commit: `40d1d4c docs: define calendar picking and answer visibility`、`abef308 test: expose calendar and answer-list gaps`、`5a83008 feat: add calendar picking and answer list`

## 結論

月間カレンダーから候補日を選ぶ機能と、回答保存後にその時点のみんなの回答を見る機能は実装済みである。

通常作成と継続作成は同じ候補pickerを使う。
時刻は文字inputで編集でき、初期値は `19:00` である。
日付buttonは現在の基準時刻との組を追加・解除し、追加済み候補は黙って時刻を変更しない。

回答保存後は、成功responseに含まれる完全な回答一覧を同じ画面へ表示する。
共有URLを開いただけのpublic projectionには回答者名と個別回答を加えず、一覧専用のpublic GET、URL parameter、cookie、localStorageも追加していない。

domain、repository、server contract、UI semantics、responsive CSS contract、自動test、Clippy、format、Fullstack build、無認証の実HTTPはPASSした。
320px・desktopの実ブラウザー操作は検証環境のbrowser接続を初期化できなかったためUNVERIFIEDであり、両Storyはin progressとして残す。

## test-firstの証拠

StoryとADRを `40d1d4c` で先にcommitした。
次に `abef308` でcalendar算術、候補toggle、初期値、通常作成と継続作成、participant matrix、participant認可、responsive contractのtestだけを追加した。

この時点では `CalendarMonth`、`toggle_calendar_candidate`、`ParticipantResponseMatrixView`、participant向けrepository readが存在せず、compile errorまたはassertion failureとしてREDになった。
実装中には、日付が空でも初期値 `19:00` がpending candidateと解釈され、未入力なのに「日付と時刻を両方入力してください」となる不具合もRED testで見つけた。
日付が空なら基準時刻だけをpending candidateと見なさないよう直し、`5a83008` で全testをGREENにした。

## 月間カレンダー

`CalendarMonth` は1年1月から9999年12月までを扱い、月移動、日数、先頭曜日、ISO日付をpure Rustで求める。
閏年、月境界、年境界、不正な日付shapeをtestで固定した。

browser local dateをhydration後に一度読み、当月を表示する。
SSRはserver timezoneを当月として推測せず、「カレンダーを準備しています…」と日付直接入力fallbackを返す。
実HTTPのroot HTMLには「候補の時刻」、`19:00`、「カレンダーを準備しています…」が含まれた。

day cellはnative `button` であり、選択時はcheck mark、色、`aria-pressed`、追加・削除を含むaccessible nameを持つ。
矢印keyを実装していないため `role="grid"` を付けず、Tab、Shift+Tab、Enter、Spaceというnative buttonの契約を保った。

CSSは7列を `repeat(7, minmax(0, 1fr))` で分ける。
320pxの既存paddingを差し引いても一日あたり約25pxを使え、高さ44pxを保つ。
page全体ではなく、候補picker自身が `min-width: 0` で縮む。

## 回答後一覧と認可境界

`POST /api/answers/submit` はresponse capabilityをhash化し、従来のidempotent保存をcommitした後にparticipant向けread snapshotを開く。
同じeventのresponse rowへ同じcapability hashが一致した場合だけ、全候補、全回答者、全○・△・×を再構成する。
別event、別capability、不在responseは同じnot-foundへ閉じる。

participantとorganizerは中立な `ResponseMatrix` projectionと再構成処理を共有するが、認可入口は分ける。
projectionにはevent名、timezone、候補日時、回答者名、回答だけを含め、response ID、candidate ID、capability、hash、account、commentを含めない。

保存後のreadだけが失敗した場合はgenericな500を返す。
clientは同じcapabilityと入力を保持するため、同一payloadを再送すると回答を二重作成せず一覧取得を再試行できる。
任意のひとことを保存またはskipしても、取得済みmatrixをcomponent stateから破棄しない。

一覧はcaption、列見出し、行見出し、記号と意味を持つtableである。
幅の広いtableだけをfocus可能なnamed region内で横scrollし、回答者列をstickyにする。

## 自動検証

```text
cargo test -q
  PASS: default 116 tests

cargo test --features server -q -- \
  --skip simultaneous_identical_retries_create_one_response \
  --skip simultaneous_identical_comments_create_one_value
cargo test --all-features -q -- \
  --skip simultaneous_identical_retries_create_one_response \
  --skip simultaneous_identical_comments_create_one_value
  PASS: 各222 tests

既知の同時再送2件をserverとall-featuresで各々単独実行
  PASS: 各構成のlogical 224 testsすべて

cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
dx build --web
git diff --check
  PASS
```

Story固有では、calendarの閏年・月移動・exact toggle・20件上限、`19:00` 初期値、空の日付と基準時刻の分離、共有picker、HTML semantics、participant認可、全回答再構成、公開projection非回帰、comment後の一覧保持、local table scrollを独立したtestで確認した。

## HTTPと稼働状態

- `127.0.0.1:8081` のrootはHTTP 200を返した。
- rootのSSR HTMLは `19:00` とhydration前のcalendar準備表示を含んだ。
- 空payloadでの `/api/answers/submit` は403となり、失敗responseにも `Cache-Control: no-store` と `X-Content-Type-Options: nosniff` が付いた。
- 実際のevent作成、calendar click、回答送信、回答後一覧のround-tripは実ブラウザー未検証である。
- 自動testでは、初回保存と同一payloadのretryがいずれも今送った回答を含むmatrixを返すことを確認した。

## designer review

sourceとHTML contractのreviewでは、page-level overflow、localized table scroll、長い回答者名、focus ring、native control、選択状態、44px高の操作、320pxでのcalendar列幅を確認した。
残存するP0/P1/P2のsource-level指摘はない。

ただし、computed `scrollWidth`、実Tab順、hover/focus/pressedの見え方、長い回答者名を含む実table scroll、screen reader announcementは実ブラウザーで確認していない。

## 実ブラウザー

次はUNVERIFIEDである。

- 320pxとdesktopのcomputed overflow、calendarの実効target幅、table領域だけの横scroll。
- 前月・次月、19:00からの時刻変更、複数候補、追加済み候補の不変性。
- keyboardだけでのTab、Shift+Tab、Enter、Spaceとvisible focus。
- 実回答POST後の「みんなの回答」、任意のひとこと保存・skip後の一覧保持。
- screen readerによる年月、pressed state、caption、row/column headerの読み上げ。

in-app Browserは、plugin runtimeのtrusted pathエラーにより初期化を二度試しても接続できなかった。
アプリのHTTP 200とは別の検証環境障害である。
以前に追加の外部browser automationは実行しない方針となっているため、別経路へ迂回しなかった。

## 一次情報

- [調整さんhelp: イベントを作成する](https://help.chouseisan.com/ja/articles/9969027-%E3%82%A4%E3%83%99%E3%83%B3%E3%83%88%E3%82%92%E4%BD%9C%E6%88%90%E3%81%99%E3%82%8B): 候補の直接入力とcalendar clickを併存させる根拠。
- [調整さん: デフォルト時刻設定](https://chouseisan.com/l/post-132401/): default時刻を利用者が変更できる設計の根拠。
- [WAI APG: Date Picker Dialog Example](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/examples/datepicker-dialog/) と [Grid Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/): calendar gridを名乗る場合のkeyboard契約を切り分ける根拠。
- [WCAG 2.2: Target Size (Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html) と [Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html): 24px幅、320 CSS px、localized two-dimensional scrollの根拠。
- [WAI: Tables with Two Headers](https://www.w3.org/WAI/tutorials/tables/two-headers/): respondent row headerとcandidate column headerを持つtableの根拠。
- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html) と [Isolation](https://www.sqlite.org/isolation.html): 保存commit後に一つのDEFERRED read snapshotでmatrixを読む根拠。
- [OWASP: Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html): capabilityをresourceごとに再認可し、public readへ広げない根拠。

## 証拠の境界

| 層 | 状態 | 証拠 |
| --- | --- | --- |
| Git | PASS | Story・ADR、初期RED、実装の三つの作業区切りcommit |
| Rust test / lint / format | PASS | default 116件、server/all-features logical 224件、Clippy、format |
| Fullstack build | PASS | Dioxus client、server成果物 |
| SQLite | PASS | participant認可、完全matrix、別capability拒否、idempotent retry |
| local HTTP | PASS | root 200、SSR初期値、private POST失敗header |
| actual calendar / answer round-trip | UNVERIFIED | browser plugin runtimeの初期化失敗 |
| Chromium 320px / desktop | UNVERIFIED | computed layoutと実操作を確認していない |
| keyboard / screen reader / physical device | UNVERIFIED | 実端末操作をしていない |
