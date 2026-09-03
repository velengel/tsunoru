# Story 0011 検証記録

Date: 2026-09-02

対象commit: `a1480c2 docs: define private event trace`、`6124037 docs: authorize read-only event traces`、`ab86455 test: specify private event trace`、`a655edf feat: project role-scoped event traces`、`c693c1f feat: open private event traces from history`、`087f7c8 test: expose event trace review gaps`、`64b407a test: expose ambiguous trace disclosures`、`afc810d fix: close event trace review gaps`

## 結論

Product Story 9の「イベントの痕跡」は実装済みである。
login中に主催または回答したeventは、短い履歴一覧からaccount-privateな詳細を開ける。
詳細は新しい活動記録を要求せず、既存のevent、候補日時、日程決定、回答者名、availability、任意のひとことを再構成する。

主催accountはそのeventに届いた全responseをreadできる。
回答しただけのaccountは、保存時に自分へ直接結び付いたresponseだけをreadできる。
両方の関係がある場合は全responseを一度だけ返し、本人のresponseを識別する。
このaccount sessionは履歴詳細のreadだけを認可し、既存の主催者summary、matrix、日程決定、回答更新のcapabilityを置き換えない。

domain、SQLite snapshot、server function、role別認可、SSR privacy、UI state、responsive contract、自動test、Clippy、format、Fullstack build、実HTTPはPASSした。
320pxとdesktopの実ブラウザー操作は未実施のため、Storyはin progressとして残す。

## test-firstの証拠

StoryとADRを先にcommitした後、`ab86455` でdomain、repository、server、UI、responsive testだけを追加した。
未実装のtype、repository function、private POST、route、componentにより、対象testは期待したcompile errorまたはassertion failureでREDになった。

最初のGREENは、`a655edf` のrole-scoped projectionと `c693c1f` のserver・UIで作った。
新しいmigrationまたは履歴snapshotへのwriteは加えず、既存aggregateを一つのread transactionで厳密に再構成した。

独立レビュー後、`087f7c8` で次の二件を修正より先にREDへした。

- 同じtyped routeでevent AからBへ遷移したとき、Aのprivate stateをBへ残さない。
- foreign keyを無効にして作った未知responseのavailability cellを、INNER JOINで検査前に消さない。

続いて `64b407a` で、同じaccountの複数responseを番号で区別することと、native disclosureのmarkerとfocus ringを失わないことをREDへした。
`afc810d` で全件をGREENにし、Story 9固有のfile-backed WAL snapshot testも追加した。

## 認証と認可

`POST /api/account/history/event-detail` はevent public IDだけをclient入力として受け取る。
account ID、role、session token、organizer capability、response capabilityをrequest bodyまたはURLから受け取らない。
unsafe APIは既存middlewareのsame-origin `Origin` または `Referer` 検査を通る。

cookieなしはguest、形が壊れた、失効済み、期限切れのcookieはexpiredとして扱う。
有効なsessionでも、event不存在とaccountが無関係なeventは同じ404と同じ一般的なbodyへ揃える。
private APIのsuccess、application error、routing error、decode errorには `no-store` と `nosniff` を付ける。

主催関係は `events.organizer_account_id`、回答関係は同じeventの `responses.respondent_account_id` からrequestごとにserverが判断する。
主催関係では全response、回答関係だけなら本人responseにqueryを限定する。
participant用のresponse queryとavailability queryの両方にaccount scopeを残すため、所属だけ確認してevent全体を読むscope wideningはない。

account sessionが追加する権限は、このevent traceのreadだけである。
既存のorganizer capabilityなしにsummary、matrix、日程決定を開いたり変更したりできない。
detail responseはsession cookieを発行も削除もしないため、古いdetail responseが並行loginの新cookieを消すこともない。

## projectionとSQLite snapshot

projectionは次の表示用情報だけを返す。

- event public ID、event名、任意の主催者のひとこと、timezone
- authoring時のposition順の候補日時
- 任意の決定した開催日時
- serverが決めた主催、回答、両方の関係
- roleから見えるresponseの回答者名、任意のひとこと、候補順のavailability、本人responseかどうか

account ID、login ID、candidate ID、response ID、`decided_at`、session、capability、token、hashは返さない。
`decided_at` は確定操作のUTC時刻であり、利用者へ示す決定済み開催日時とは異なるためprojectionへ入れていない。

sessionの期限判定、必要なtouch、期限切れ削除は短い `BEGIN IMMEDIATE` でcommitする。
その後、account関係の認可SELECTを最初のreadにしたDEFERRED transactionで、event、候補、決定、scope済みresponse、availability、commentを読む。

file-backed WAL testでは、認可でsnapshotを確立した後に別connectionからresponseをcommitした。
進行中のdetailは古い完全snapshotとして0件を返し、新しいtransactionだけが1件を返した。
異なる時点のevent、response、cellを一画面へ混ぜない。

候補はposition順、responseは内部ID順へ安定させるが、内部IDをclientへ返さず、ID順を保存時刻とは呼ばない。
全responseと全候補の直積を検査し、欠損、重複、未知candidate、未知response、未知availability、件数overflowでは部分結果を返さない。
organizer queryはLEFT JOINで未知response cellも検査地点まで保持し、participant queryは他accountのcellを返さない。

新しいmigrationはない。
現在のevent、候補、response、comment、decisionが実質immutableであるため、既存aggregateを「当時」の正本として読む。
将来これらの編集または削除を導入する前に、revisionまたはsnapshotを再判断する必要がある。

## UIとSSR

`/history/events/:public_id` はSSR時にgenericなloading shellだけを返す。
event名、回答者名、availability、commentはhydration後の `use_effect` から取得し、`use_server_future` またはloaderのSSR payloadへ入れない。
HTMLの `/history` と `/history/events/*` にも `no-store`、routeには `noindex,nofollow` を指定する。

一覧には従来のevent名、決定、回答件数だけを残し、「当時の記録を見る」linkを加えた。
回答者名、availability、comment previewは一覧へ追加していない。
detail末尾からpublicな共有ページへ移れる。

detailはloading、guest、expired、missing、failure、未決定、response 0件、表示成功を区別する。
自分のresponseは展開し、主催者として見えるその他のresponseはnative `details` へまとめる。
availabilityは記号だけでなく「行ける」「条件次第」「難しい」をtextで示す。
同じaccountのresponseが複数ある場合は「回答 1 / 2」のような安定した番号を付ける。

routeのpublic IDは `use_reactive` のdependencyにし、さらにpublic IDをkeyにしたroute contentをremountする。
event AからBへSPA遷移した最初のpaintでAを残さず、Aの後着responseもBのstateへ採用しない。
retryにもrequest generationを使い、最新requestだけがstateを更新する。

mobileでは一列、候補回答だけをdesktopで二列にし、520px以下で一列へ戻す。
長いevent名、回答者名、ひとことを折り返し、操作領域を44px以上にする。
custom disclosure markerを表示し、summaryの外側focus ringを親のoverflowでclipしない。

写真、後日の感想、reaction、timeline、series、名前解析、次回名の提案、cross-event groupingはtype、query、route、UIへ加えていない。

## 自動検証

```text
cargo test
  PASS: default 93 tests

cargo test --features server -- \
  --skip simultaneous_identical_retries_create_one_response \
  --skip simultaneous_identical_comments_create_one_value
cargo test --all-features -- \
  --skip simultaneous_identical_retries_create_one_response \
  --skip simultaneous_identical_comments_create_one_value
  PASS: 各190 tests

既知の同時再送2件をserverとall-featuresで各々単独実行
  PASS: 各構成のlogical 192 testsすべて

cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
dx build --web
git diff --check
  PASS
```

Story 9の最終focused suiteは、SQLite unit 3件、auth 9件、domain 3件、repository 5件、server 3件、UI 4件、既存account UI 10件、responsive 10件の計47件がPASSした。

## HTTPと稼働状態

- `127.0.0.1:8081` と `127.0.0.1:8082` は同時に稼働し、rootはHTTP 200を返した。
- `/history/events/:public_id` のSSRはHTTP 200、`no-store`、`nosniff`、`noindex,nofollow` を持ち、genericなloadingだけを含んだ。
- cookieなしのdetail POSTはHTTP 200の `guest`、`no-store`、`nosniff` を返し、`Set-Cookie` はなかった。
- local test accountでeventを主催すると、detailはrelationship `organized`、候補2件、response 0件を返した。
- 別accountで回答とひとことを保存すると、participant detailは本人response一件だけを `participated` として返し、organizer detailは同じresponseを `organized` として返した。
- organizer projectionではparticipant responseの `is_current_account` はfalse、participant projectionではtrueだった。
- 無関係accountから既存eventを開いた場合と、存在しないeventを開いた場合は、同じ404、同じbody digestだった。
- organizer、participant、missingのdetail responseはいずれも `Set-Cookie` を返さなかった。
- 実HTTPで使ったsession cookieとcapabilityを含む一時fileはterminalへ値を出さず、検証後に削除した。

## 独立レビュー

Dioxus・auth、SQLite・projection、mobile UX・accessibilityの三方向でread-onlyレビューを実施した。

最初のreviewでは、同じrouteのpublic ID変更がeffect dependencyになっていないrace、INNER JOINが未知response cellを隠すこと、候補順と両方roleのtest不足、Story 9固有のWAL snapshot test不足を指摘された。
routeをreactiveかつkeyedにし、organizer cellをLEFT JOINで検査へ残し、candidate positionとavailability対応、本人・別account・anonymousの全scope、WAL競合をtestへ追加した。

UX reviewでは、`overflow: clip` がsummaryのfocus outlineを欠かせること、flex summaryがnative markerを失うこと、同名・同payloadの複数responseを見分けられないことを指摘された。
focus ringをclipしないborder、明示的なmarker、response ordinalをREDから追加した。

修正後は三方向で再reviewした。
Dioxus・authとSQLite・projectionでは既知の指摘が解消され、新規P0/P1なしを確認した。
mobile UX・accessibilityでも既知のP0/P1が解消され、新規P0/P1はなかった。
route AからBへの遷移は、source contractとhook semanticsでは閉じたが、遅延responseを発生させる実ブラウザー回帰testは未実施というP2のtest強度を残す。

## 実ブラウザー

次はUNVERIFIEDである。

- 320pxとdesktopのcomputed overflow、候補回答の二列から一列へのreflow、長いevent名、回答者名、500文字comment。
- keyboardだけで履歴、detail、response disclosure、public event、戻る、retryを操作できること。
- success、missing、failure、route AからBへの遷移後の実DOMと `activeElement`。
- screen readerのstatus、details、response ordinalの読み上げ。

利用可能なin-app browser clientは起動できず、外部Playwright実行の追加承認も得ていない。

## 一次情報

- [Dioxus 0.7: Authentication](https://dioxuslabs.com/learn/0.7/essentials/fullstack/authentication/): authentication後もdataごとにauthorizationする根拠。
- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/): typedなAxum endpointとserver-only request contextの根拠。
- [Dioxus 0.7: SSR](https://dioxuslabs.com/learn/0.7/essentials/fullstack/ssr/): server futureの値がSSRとhydration向けに直列化される境界の根拠。
- [Dioxus 0.7: Middleware](https://dioxuslabs.com/learn/0.7/essentials/fullstack/middleware/): session依存routeのcache境界をrouterで揃える根拠。
- [Dioxus Hooks 0.7.10: use_reactive](https://docs.rs/dioxus-hooks/0.7.10/dioxus_hooks/fn.use_reactive.html): 通常propをeffect dependencyへ変換する根拠。
- [OWASP Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html): deny by defaultとrequestごとのauthorizationの根拠。
- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html) と [SQLite: Isolation](https://www.sqlite.org/isolation.html): DEFERRED read transactionのsnapshotとWAL競合testの根拠。
- [WHATWG HTML: The details element](https://html.spec.whatwg.org/multipage/interactive-elements.html#the-details-element): native disclosure semanticsの根拠。
- [WCAG 2.2: Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html) と [Focus Visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html): 320 CSS pxとkeyboard focusの根拠。

## 証拠の境界

| 層 | 状態 | 証拠 |
| --- | --- | --- |
| Git | PASS | Story、ADR、初期RED、data、server、UI、review RED、review修正の作業区切りcommit |
| Rust test / lint / format | PASS | default 93件、server各logical 192件、focused 47件、Clippy、format |
| Fullstack build | PASS | Dioxus client、server成果物 |
| local HTTP | PASS | 8081/8082、private SSR、guest、organizer、participant、同一404、cookie非変更 |
| SQLite | PASS | role scope、全cell、不変条件、WAL snapshot、migration不要 |
| Chromium 320px / desktop | UNVERIFIED | browser client起動不良、外部script未承認 |
| keyboard / screen reader / physical device | UNVERIFIED | 実端末操作をしていない |
