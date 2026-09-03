# Story 0012 検証記録

Date: 2026-09-02

対象commit: `29a11e5 docs: define explicit event continuation`、`39bb003 docs: decide explicit series membership`、`28d1c75 test: specify explicit event continuation`、`9ee484b feat: continue explicit event series`、`26ebcc3 docs: harden continuation recovery boundaries`、`dd11db7 test: expose continuation recovery gaps`、`61a8303 fix: harden continuation recovery`、`efb89ca docs: retain continuation recovery hints`、`66cc379 test: expose lost continuation hint`、`104451a fix: retain continuation recovery hint`、`8cb73fb test: preserve one-time capability response`、`12ed444 docs: serialize continuation recovery actions`、`d4866c5 test: expose concurrent recovery actions`、`a1b7b18 fix: serialize continuation recovery actions`

## 結論

Product Story 10の継続イベントと命名補助は実装済みである。
login中の主催者は、自分が主催したeventのprivate履歴詳細からだけ「同じ活動の次回をつのる」を開始できる。
通常の匿名作成、名前の類似、参加履歴からseriesを自動作成しない。

末尾名が厳密な `base + " #" + positive ASCII N` の場合だけ、checkedな `N + 1` を編集可能な候補にする。
名前を編集または削除しても、明示的なmembershipから同じ活動へ追加する。
並行する二つのcontinuationは一件だけをcommitし、もう一件をstaleへ止める。

domain、migration、repository、private server function、UI、responsive contract、自動test、Clippy、format、Fullstack build、無認証の実HTTPはPASSした。
認証付き実HTTP round-tripと320px・desktopの実ブラウザー操作は未実施のため、Storyはin progressとして残す。

## test-firstの証拠

StoryとADRを先にcommitした後、`28d1c75` でdomain、repository、server、UI、responsive testだけを追加した。
未実装のtype、function、route、schema、CSSにより、compile errorまたはassertion failureとしてREDになった。

初回実装後の独立レビューでは、capabilityの自動 `Debug`、plan read中のwriter lock、並行・途中失敗・破損dataのtest不足、validation focus、409後の最新候補、401からの復旧、series disclosure markerを指摘された。
`26ebcc3` で判断を先にADRへ記録し、`dd11db7` で修正前のRED testを追加してから、`61a8303` でGREENにした。

最終再レビューでは、最新候補を表示した後の再送開始時に、その候補を結果確定前に捨てるP2が見つかった。
再送が401または一時的な500になるとdraftだけが残り、文言と復旧操作が食い違っていた。
`efb89ca` で候補stateの寿命を決め、`66cc379` でREDにし、`104451a` で新しい409だけが候補を無効にするよう修正した。
さらに送信中にも候補適用buttonを押せる競合と、新しい409で古い候補説明が優先される不整合を見つけた。
`12ed444` で操作順を決め、`d4866c5` でREDにし、`a1b7b18` で送信・再読込中の候補適用を無効にしてstale文言を優先した。

## seriesと命名補助

series membershipはevent名から推測せず、主催者がprivate履歴詳細からcontinuationを開始した事実として保存する。
最初のcontinuationで起点をposition 0、新eventをposition 1へ入れ、以後はcurrent tailの次へ追加する。
履歴ではseries内を新しいeventから表示し、単発の主催履歴と参加履歴は分けたままにする。

次回名を提案するのは、末尾が厳密な半角 ` #N` で終わる場合だけである。
全角記号、全角数字、先頭0、`#0`、途中の番号、整数overflow、100文字overflowには提案しない。
通常の `飲み会` から `#18` のような番号を作らない。
利用者が明示的に `飲み会 #17` を続ければ `#18` は候補になり得るが、自由に編集・削除できる。

## 認証と公開境界

planは起点eventのpublic IDだけを受け取り、createは起点public ID、expected tail public ID、通常と同じevent inputだけを受け取る。
account ID、series ID、role、session、主催者capabilityをclient指定へしない。

serverはrequestごとにsessionと `events.organizer_account_id` を確認する。
未login、失効、回答しただけ、別account、anonymous eventはseries authorityにならない。
createでsessionが失効しても通常のanonymous createへfallbackせず、eventを保存しない。

private planとcreateは `no-store`、`nosniff` を返し、session cookieを発行も削除もしない。
SSRはgenericなloading shellだけを返し、起点名、series名、次回名をHTMLへ含めない。
series内部ID、account ID、position、session、capability、token、hashはplanまたは履歴projectionへ返さない。

作成成功だけは従来どおり生の主催者capabilityを一度browserへ返す。
SQLiteにはhashだけを保存し、`CreatedEvent` のcustom `Debug` は生値を `[REDACTED]` にする。
回帰testはdebugに生値がないことと、JSON success responseには保存用の生値が残ることを別々に確認する。

## SQLiteとatomicity

migration 0007は `event_series` と `event_series_members` を追加した。
一event一series、series内position一意に加え、series ownerとevent organizerが同じaccountであることを二つの複合foreign keyで固定する。

continuation createは一つの `BEGIN IMMEDIATE` でactive session、起点owner、current tailを再検証し、series、起点membership、event、全candidate、新membershipを保存する。
expected tailが古ければ409としてwrite前に止める。
candidate INSERTの途中失敗では、先に作ったseriesとeventを含めて全rowがrollbackした。

file-backed SQLiteの同時testでは、同じtailから二要求を開始し、成功一件と `Stale` 一件になった。
seriesは一件、memberは起点と成功した新eventの二件だけで、分裂または部分eventは残らなかった。

planはsessionの期限判定と必要なtouchだけを短いwrite transactionで終え、owner、series、全member、tailをDEFERRED read snapshotで読む。
長いseriesを確認する間、匿名作成と回答が必要とするwriterを予約しない。
create側が保存直前にすべて再検証するため、planとcreateの間の変更はstale検査で閉じる。

account削除ではseriesとmembershipをcascade deleteし、eventのownerをNULLにしてpublic aggregateを残した。
空series、一memberだけ、消失event、owner不一致、position不連続は部分表示せずdata invariant違反にする。

## UI

履歴詳細には主催関係がある場合だけcontinuation linkを置く。
continuation routeはloading、guest、expired、missing、failure、候補あり、候補なし、validation、stale、成功を区別する。
通常作成へ戻るlinkを常に示し、候補名の削除とseries continuationの中止を同じ操作にしない。

409後の再読込は、利用者の名前、ひとこと、候補日時を上書きしない。
最新候補を別のpanelへ示し、「最新の候補を使う」を選んだときだけevent名へ反映する。
再送が401または一時的な失敗でもそのpanelを残し、新しい409で古くなった場合だけ消す。
createまたはplan再読込中は候補適用buttonを無効にし、request確定後に画面だけ名前が変わらないようにする。

送信時のsession失効は、draftを同じtabへ残し、loginを別tabで開くlinkを示す。
validation失敗は描画後に最初の該当fieldへfocusする。
series履歴はnative `details` / `summary` / `ul` を使い、flex layoutでも見えるmarkerとfocus ringを明示する。

CSS contractでは320pxで一列、退出linkの縦積み、44px以上の操作、長文折返しを確認した。
computed layout、実Tab順、実focus、screen reader announcementは実ブラウザー未検証である。

## 自動検証

```text
cargo test
  PASS: default 108 tests

cargo test --features server -- \
  --skip simultaneous_identical_retries_create_one_response \
  --skip simultaneous_identical_comments_create_one_value
cargo test --all-features -- \
  --skip simultaneous_identical_retries_create_one_response \
  --skip simultaneous_identical_comments_create_one_value
  PASS: 各213 tests

既知の同時再送2件をserverとall-featuresで各々単独実行
  PASS: 各構成のlogical 215 testsすべて

cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
dx build --web
git diff --check
  PASS
```

Story 10固有では、strict parser、private projection、owner認可、composite foreign key、並行stale、途中rollback、破損data、通常flow非回帰、全UI state、latest suggestion、responsive selectorを独立したtestで確認した。

## HTTPと稼働状態

- `127.0.0.1:8081` と `127.0.0.1:8082` は同時に稼働し、rootはHTTP 200を返した。
- `/history/events/:public_id/continue` のSSRは200、`no-store`、`nosniff` で、privateなpublic IDとevent名を含まずgeneric loadingだけを返した。
- cookieなしのplan POSTは200の `guest`、`no-store`、`nosniff` を返し、`Set-Cookie` はなかった。
- cookieなしのcreate POSTはwrite前に401、`no-store`、`nosniff` を返し、`Set-Cookie` はなかった。
- 認証付きround-tripのためのtest account作成は、認証dataとsession cookieをSQLiteへ追加する外部操作として実行許可を得られなかった。迂回せずUNVERIFIEDとした。
- organizerだけのplan/create、別accountとparticipantの404、stale 409、grouped historyはisolated SQLite repository testでPASSした。
- HTTP検証の一時fileは値をterminalへ表示せず、検証後に列挙した正確なpathだけを削除した。

## 独立レビュー

Dioxus・auth、SQLite・不変条件、mobile UX・accessibilityの三方向でread-onlyレビューを行った。

初回レビューのP1は、生capabilityの自動 `Debug` と、series summaryの消えたmarker、validation focus、409後の候補不足、401からの復旧であった。
plan readのwriter lockと並行・rollback・corruption test不足もP2として修正した。

再レビューでは、SQLite側の指摘はすべて解消し、atomic createと受容済みのplan/logout raceを確認した。
Dioxus・auth側もcustom DebugとSerialize、plan readとcreate再認可、別accountのfail-closedを確認した。
UX側が最新候補stateの早すぎる破棄と、送信中にも候補を適用できる競合を追加で見つけ、いずれもADRとREDを先に追加して修正した。
最終レビューでは、SQLite、Dioxus・auth、mobile UX・accessibilityのいずれにも残存するP0/P1/P2がないことを確認した。

残るP2は、409、別tab login、route切替を実際に発生させるbrowser testがなく、source contract中心であること。
無認証の実HTTPは確認したが、認証付きround-tripも未検証である。

## 実ブラウザー

次はUNVERIFIEDである。

- 320pxとdesktopのcomputed overflow、series marker、長い起点名、最新候補panel。
- keyboardだけで履歴詳細、continuation、名前編集、候補日時、通常作成への退出、別tab login、stale recoveryを操作できること。
- validation、401、404、409、500、成功後の実DOMと `activeElement`。
- route AからBへの遷移、out-of-order response、screen readerのstatusとdetails読み上げ。

利用可能なin-app browser clientは起動できず、外部browser automationの追加実行も認められていない。

## 一次情報

- [Dioxus 0.7: Fullstack](https://dioxuslabs.com/learn/0.7/essentials/fullstack/): clientとserverの一体構成を選ぶ根拠。
- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/): private typed POSTとserver-only request contextの根拠。
- [Dioxus 0.7: Authentication](https://dioxuslabs.com/learn/0.7/essentials/fullstack/authentication/): authentication後もresourceごとにauthorizationする根拠。
- [Dioxus Hooks 0.7.10: use_reactive](https://docs.rs/dioxus-hooks/0.7.10/dioxus_hooks/fn.use_reactive.html): route public IDをreactive dependencyへ変える根拠。
- [SQLite: Foreign Key Support](https://www.sqlite.org/foreignkeys.html): composite foreign keyとdelete actionの根拠。
- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html) と [Isolation](https://www.sqlite.org/isolation.html): write直列化、rollback、read snapshotの根拠。
- [Rust: `std::fmt::Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html): secretを自動debug表示から分ける根拠。
- [OWASP: Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html): requestごとのowner確認とdeny by defaultの根拠。
- [WHATWG HTML: The details element](https://html.spec.whatwg.org/multipage/interactive-elements.html#the-details-element): native disclosure semanticsの根拠。
- [WCAG 2.2: Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html)、[Focus Order](https://www.w3.org/WAI/WCAG22/Understanding/focus-order.html)、[Focus Visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html): 320 CSS px、error focus、visible markerの根拠。

## 証拠の境界

| 層 | 状態 | 証拠 |
| --- | --- | --- |
| Git | PASS | Story、ADR、初期RED、実装、review ADR、review RED、修正の作業区切りcommit |
| Rust test / lint / format | PASS | default 108件、server/all-features logical 215件、Clippy、format |
| Fullstack build | PASS | Dioxus client、server成果物 |
| SQLite | PASS | migration、複合FK、owner、並行stale、atomic rollback、account delete、corruption fail-closed |
| local HTTP without auth | PASS | 8081/8082、private SSR、guest plan、unauthenticated create、cookie非変更 |
| local HTTP with auth | UNVERIFIED | test accountとsessionを作る実行許可なし |
| Chromium 320px / desktop | UNVERIFIED | browser client起動不良、外部automation未実行 |
| keyboard / screen reader / physical device | UNVERIFIED | 実端末操作をしていない |
