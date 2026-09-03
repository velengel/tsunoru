# ADR 0016: account関連からread-onlyなイベントの痕跡だけを認可する

Status: accepted

Date: 2026-09-02

Amends: [ADR 0015](0015-keep-account-history-optional-and-server-session-bound.md)

## context

Product Story 8のaccount履歴は、event名、決定した開催日時、主催履歴の回答件数だけを返す。
回答者名、候補ごとの回答、ひとことを一覧へ含めないため、過去eventへ戻れても、調整の過程で自然に生まれた痕跡までは振り返れない。

Product Story 9は、新しい活動記録を書かせず、決定した開催日時、回答者名、当時の回答、当時のひとことを詳細でだけ確認できるようにする。
写真、後日の感想、reaction、timelineは要求せず、Story 10のseriesと次回名の提案も先取りしない。

現在のSQLiteには、event、候補、日程決定、response、候補ごとのavailability、nullableなrespondent commentが既に保存されている。
login中に作成したeventは `organizer_account_id`、login中に初めて保存したresponseは `respondent_account_id` へ、元のwriteと同じtransactionで結び付く。
同名の回答者や後から行ったloginを根拠に、過去のanonymous responseをaccountへ推測して結び付けることはない。

ADR 0015ではaccount sessionを主催者capabilityまたは回答capabilityの代わりにせず、履歴からpublic eventへだけ戻すことを決めた。
Story 9でaccount履歴からprivateな痕跡を読むには、この境界を狭く拡張し、accountとの関係がどのreadを認可するかを明示する必要がある。
この判断を曖昧にすると、参加者が他人の回答やひとことを読んだり、account sessionだけで日程決定を変更したりする権限拡大につながる。

private detailをSSRで解決すると、responseがHTMLまたはhydration dataへ直列化され、共有端末のsource、cache、戻る操作へ残りやすい。
また、session判定は期限切れrowの削除と一定間隔の `last_seen_at` 更新を行い得るため、単なる静的GETではない。

参考資料:

- [First Instruction: 履歴](../../first-instruction.md#11-履歴)
- [Dioxus 0.7: Authentication](https://dioxuslabs.com/learn/0.7/essentials/fullstack/authentication/)
- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/)
- [Dioxus 0.7: SSR](https://dioxuslabs.com/learn/0.7/essentials/fullstack/ssr/)
- [Dioxus 0.7: Middleware](https://dioxuslabs.com/learn/0.7/essentials/fullstack/middleware/)
- [OWASP: Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html)
- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html)
- [SQLite: Isolation](https://www.sqlite.org/isolation.html)
- [WHATWG HTML: The details element](https://html.spec.whatwg.org/multipage/interactive-elements.html#the-details-element)
- [WCAG 2.2: Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html)

## decision

- `/history/events/:public_id` をaccount-privateな履歴詳細routeとして追加する。履歴一覧のevent名、決定、回答件数は増やさず、各項目からこの詳細へ進む。public eventへの導線は詳細にも残す。
- private detailはSSRで解決しない。SSRはevent名、回答者名、回答、ひとことを含まないgenericなloading shellだけを返し、hydration後の `use_effect` から明示的に取得する。`use_server_future` またはloaderでprivate projectionをSSR payloadへ入れない。
- detail routeへ `noindex,nofollow` を指定する。`/history` と `/history/events/*` のHTML response、account APIのsuccess、application error、routing error、decode errorへ `Cache-Control: no-store` を付ける。APIには `X-Content-Type-Options: nosniff` も付ける。
- typed server functionは `POST /api/account/history/event-detail` とし、入力はevent public IDだけにする。account ID、role、session、organizer capability、response capabilityをclient入力として受け取らない。private readに伴うsession touchまたは期限切れ削除をunsafe methodのsame-origin検査内へ置く。
- event public IDは既存のdomain規則で検証し、壊れた入力は422にする。cookieなしはguest、形が壊れた、失効済み、期限切れのsessionはexpiredとして区別する。
- 有効なsessionでも、event不存在とaccountに閲覧関係がない場合は同じ404と同じ一般的な文言にする。DB errorとdata invariant違反は、内部ID、SQL、accountの有無を含まない一般的な500にする。
- account関連をStory 9専用のread認可根拠として使う。`events.organizer_account_id` が現在のaccountなら主催関係、同じeventに `responses.respondent_account_id` が現在のaccountであるresponseが一件以上あれば回答関係とする。requestごと、eventごとにserverで判定する。
- 主催関係では、そのeventに届いた全responseの回答者名、全候補のavailability、任意のひとことを読める。主催と回答の両方なら全responseを一度だけ返し、現在のaccountへ直接結び付いたresponseを表示上識別できるようにする。
- 回答関係だけなら、現在のaccountへ直接結び付いたresponseだけを返す。同じaccountが同じeventへ複数回答していれば、最新一件を推測して選ばず、すべてを別の痕跡として返す。回答者名またはpayloadが同じでもdeduplicateしない。
- anonymous response、別accountのresponse、login前または期限切れsession中に保存したresponseを、回答者名やlogin IDから現在のaccountのresponseと推測しない。
- account sessionが認可するのはこのread-only projectionだけである。既存の主催者summary、response matrix、日程決定、回答更新、capability復旧を認可せず、organizer capabilityとresponse capabilityを置き換えない。
- projectionはevent public ID、event名、任意の主催者のひとこと、timezone、候補日時、任意の決定した開催日時、serverが決めた関係、可視範囲内のresponseを含む。各responseは回答者名、任意のひとこと、候補順のavailability、現在のaccountに直接結び付いたかだけを含む。
- projectionへaccount ID、login ID、candidate ID、response ID、`decided_at`、session、capability、token、hashを含めない。`event_decisions.decided_at` は確定操作のUTC時刻であり、「決定した日時」として表示する開催候補のlocal date/timeとは別なので返さない。
- 候補はauthoring時の `position ASC`、responseは内部ID `ASC` の安定順で組み立てる。ただし内部IDをprojectionへ返さず、ID順を保存時刻とは呼ばない。
- responseごとに全候補のavailabilityが一件ずつ存在することを厳密に検証する。欠損、重複、未知の候補、未知のresponse、未知のavailability、件数計算のoverflowがあれば部分結果を返さずdata invariant違反にする。
- sessionの期限判定、必要なtouch、期限切れrow削除は、既存と同じ短い `BEGIN IMMEDIATE` transactionで完了する。その後、認可、event、候補、決定、scope済みresponse、availability、commentを一つのDEFERRED read transactionで読む。
- 認可後のresponse queryとavailability queryにもaccount scopeを保つ。回答関係だけを確認してからevent全体を読む実装にしない。
- 現在のaggregateを正本として読み、新しいmigration、activity log、履歴snapshotへの二重書きを追加しない。event、候補、response、ひとこと、決定の編集または削除を将来導入する前に、「当時」を保つrevisionまたはsnapshotを再判断する。
- 初期実装は一eventの可視responseをすべて遅延取得する。実測なしにpagination、検索、cross-event groupingを加えない。
- UIはloading、guest、expired、missing、失敗、未決定、response 0件、表示成功を区別する。非同期成功後はdetail見出しへfocusし、retryとroute変更の後着responseが古いprivate dataを表示しないようrequest generationで破棄する。
- 自分のresponseは展開して表示し、主催関係で見えるその他のresponseは回答者ごとのnative `details` にまとめる。availabilityは記号だけでなく意味をtextで示し、ひとことなしも明示する。回答、comment、決定を変更するinputまたはactionは置かない。
- mobileではresponseを一列にし、長いevent名、回答者名、ひとことを折り返す。320 CSS pxでpage全体の横scrollを発生させず、標準のfocus semanticsと44px以上の操作領域を保つ。
- 写真、感想投稿、reaction、timeline、series ID、名前解析、類似event判定、次回名の候補、cross-event groupingをStory 9のtype、query、route、UIへ入れない。

## rejected options

### public eventへ回答者名、回答、ひとことを追加する

public URLだけで痕跡へ戻れる。
しかし、URLを知る全員へ個人の回答とひとことを広げ、Story 8までのpublic projectionを変えるため採用しない。

### 履歴一覧のprojectionへ全痕跡を追加する

detail requestを省ける。
しかし、一覧を情報で埋め、初期payloadとmobileの可読性を悪化させる。明示的にdetailを開いたときだけ取得する。

### private projectionをSSRする

初回HTMLだけで内容を表示できる。
しかし、private dataをHTMLとhydration payloadへ直列化し、cacheと共有端末へ残す範囲を広げるため採用しない。

### account sessionへ既存の主催者capabilityを復元する

履歴からsummary、matrix、日程決定へ直接戻れる。
しかし、read-onlyな痕跡の追加をmutation権限の拡大に変え、eventごとのbearer authorityという既存境界を壊すため採用しない。

### 主催accountにも本人のresponseだけを返す

account sessionが読める情報を最小にできる。
しかし、主催履歴の詳細から日程調整時に届いた痕跡を振り返れず、Story 9の回答者名、当時の回答、ひとことを満たせない。主催関係へ全responseのreadだけを明示的に認可する。

### 回答関係だけでevent全体のresponseを返す

同じdetail型を単純に使える。
しかし、一参加者が他人の回答者名、回答、ひとことを読めるため採用しない。

### detailを開くたびにorganizer capabilityまたはresponse capabilityを再入力させる

既存のbearer authorityだけでprivate dataを認可できる。
しかし、別端末で使うaccount履歴の追加価値を失い、保存時のaccount関連を使う目的に合わない。

### 回答者名またはlogin IDで過去responseをclaimする

Story 8以前またはlogin前の痕跡も多く表示できる。
しかし、同名の別人と共有端末から回答を奪えるため採用しない。

### 同じeventの最新responseだけを返す

表示量を小さくできる。
しかし、複数responseが保存された事実と異なる痕跡を見せる。response ID、名前、payloadのいずれでも暗黙にまとめない。

### 既存のorganizer summaryとmatrixをclientで合成する

新しいrepository queryを減らせる。
しかし、二つの認可とsnapshotをまたぎ、matrixにひとことがなく、account sessionへorganizer capabilityを渡す必要も生じるため採用しない。

### 汎用activity logまたはsnapshot tableへ二重書きする

将来の編集後も「当時」を保持しやすい。
しかし、現在のaggregateは実質immutableであり、正本とlogの部分成功を新しく作る。編集要件が現れる前には導入しない。

### detail全体を `BEGIN IMMEDIATE` で読む

read中のwriteを防げる。
しかし、回答が多いdetailの間、anonymous event作成と回答が必要とするwriterを占有する。短いsession transactionと一つのDEFERRED read snapshotに分ける。

### 最初からpaginationまたはseries groupingを導入する

巨大eventや継続eventを先に扱える。
しかし、実測のない複雑さを増やし、seriesと命名補助を扱うStory 10を先取りするため採用しない。

## consequences

- login中に作成したeventでは、別端末でもaccount sessionから日程調整時の全responseをreadできる。盗まれたaccount sessionが読めるprivate dataの範囲はStory 8より広がる。
- login中に回答したeventでは、そのaccountへ最初の保存時に直接結び付いたresponseだけをreadできる。他参加者のresponseは返らない。
- 同じaccountが主催と回答の両方に該当する場合、主催関係による全体と自分のresponseを一画面で区別できる。二つのprojectionを重ねて同じresponseを重複表示しない。
- Story 8以前、login前、session期限切れ後のanonymous rowはaccount履歴詳細へ現れない。後から同名accountを作っても回収されない。
- account削除時はnullable foreign keyが `SET NULL` になり、public aggregateは残るが、そのaccountとの履歴関係は失われる。event削除時は関連responseとdecisionも消えるため、永続archiveではない。
- ひとことの `NULL` は、skip、未入力、画面を閉じた状態を区別しない。UIではすべて「ひとことなし」と扱う。
- 現在のaggregateが実質immutableである間は「当時」の痕跡として読める。将来、event名、候補、response、comment、decisionを変更または削除できるようにすると、表示が過去時点を保証しなくなる。
- session認証とread snapshotは別transactionなので、認証が成立した後にlogoutと競合した一requestは完了し得る。長いdetail readがanonymous writeのwriterを占有しない方を優先する。
- 一eventの全responseを返す主催detailは大きくなり得る。初期実装は明示的な遅延取得と一列表示で受け入れ、計測後にkeyset paginationを再検討する。
- route変更、logout、retryと遅いresponseが競合するclient stateを管理する必要がある。後着responseを捨てる実装とtestが増える。
- account sessionは主催者用の変更操作を認可しないため、履歴詳細から日程決定を変更するには、従来どおりそのbrowserに保存されたorganizer capabilityが別途必要になる。
- 新しいtableとwriteは増えず、anonymous event作成と回答のoperation countも変わらない。
- Story 10はこのdetail projectionをseriesとしてまとめる前提にせず、継続eventと命名補助を別のStoryとADRで判断できる。
