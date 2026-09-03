# ADR 0017: 明示したaccount continuationからだけseriesを作る

Status: accepted

Date: 2026-09-02

Amends: [ADR 0015](0015-keep-account-history-optional-and-server-session-bound.md)、[ADR 0016](0016-authorize-read-only-event-traces-from-account-links.md)

## context

Product Story 10は、`ベストユニゾン #1` から `ベストユニゾン #2` のような次回名を補助し、同じ活動の履歴をまとめて辿れることを求める。
同時に、初回eventでseries登録を要求せず、すべてのeventを自動分類せず、`飲み会 #18` のような不自然な自動提案を避けなければならない。

現在の通常作成はloginを前提にせず、最小のevent名、任意のひとこと、候補日時だけを受け取る。
login中の作成は、同じtransactionで `events.organizer_account_id` へaccountを結び付けるが、sessionが失効していてもanonymous eventの作成自体は成立させる。
account履歴は主催と参加を分け、privateなevent traceのreadだけをaccount関係から認可する。

名前の一致をseries identityにすると、表記を少し変えただけで履歴が分裂する。
反対に、名前の類似度から自動でmembershipを保存すると、別の活動を誤ってまとめ、利用者が行っていない意思決定をsystemが作る。
初期実装には既存seriesを分割、統合、renameする管理画面もないため、誤った自動分類を後から安全に直せない。

次回名の提案も、数字を含むすべての名前から規則を推測すると誤提案が増える。
ただし、主催者がprivate履歴詳細から「同じ活動の次回をつのる」と明示した後なら、活動が続くという意図は既に入力されている。
この狭い場面でだけ、末尾の明白な ` #N` を編集可能な初期値へ変換すれば、通常flowへ分類作業を持ち込まずに補助できる。

参考資料:

- [First Instruction: シリーズ / 継続イベント](../../first-instruction.md#12-シリーズ--継続イベント)
- [Dioxus 0.7: Fullstack](https://dioxuslabs.com/learn/0.7/essentials/fullstack/)
- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/)
- [Dioxus 0.7: Authentication](https://dioxuslabs.com/learn/0.7/essentials/fullstack/authentication/)
- [Dioxus Hooks 0.7.10: use_reactive](https://docs.rs/dioxus-hooks/0.7.10/dioxus_hooks/fn.use_reactive.html)
- [OWASP: Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html)
- [SQLite: Foreign Key Support](https://www.sqlite.org/foreignkeys.html)
- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html)
- [W3C WAI: Understanding On Input](https://www.w3.org/WAI/WCAG22/Understanding/on-input.html)
- [W3C WAI: Understanding Input Assistance](https://www.w3.org/WAI/WCAG22/Understanding/input-assistance)

## decision

- 通常の `/` と `POST /api/events/create` はseriesを知らないまま残す。通常作成、public event、匿名回答、回答完了、履歴一覧を開いただけの状態では、名前解析、series候補、次回名を取得または表示しない。
- login中に主催したeventのprivate履歴詳細へ「同じ活動の次回をつのる」を置く。主催関係は `events.organizer_account_id` からrequestごとにserverで確認する。回答関係だけ、anonymous event、別accountのeventからは開始できない。
- private routeは `/history/events/:public_id/continue` とする。SSRは起点名、series名、次回名を含まないgenericなloading shellだけを返し、`noindex,nofollow` と `no-store` を既存のaccount route境界で適用する。
- continuation planは `POST /api/account/history/event-continuation`、作成は `POST /api/account/history/event-continuation/create` という別のprivate server functionにする。planのclient入力は起点eventのpublic IDだけとし、account ID、role、series ID、session、capabilityを受け取らない。cookieなし、失効、missing、無関係を既存のprivate履歴詳細と同じ非公開境界で扱う。
- planは起点event名、既存seriesなら表示名と末尾event、次回名の候補、末尾event public IDを返す。seriesの内部ID、account ID、membership position、authority materialは返さない。末尾public IDは秘密ではなく、create時のstale plan検査にだけ使う。
- 起点がまだseriesへ属さない場合は起点自身を末尾とする。既存seriesなら、利用者が古いeventから開始してもmembership positionが最大のeventを末尾として使う。
- 次回名の候補は、末尾名が厳密に `base + " #" + N` の形である場合だけ作る。`base` は非空で末尾に余分なwhitespaceを持たず、`N` は先頭0のない半角ASCII数字の正整数とする。`N + 1` はchecked演算し、結果がevent名の文字数上限以内の場合だけ返す。
- 数字なし、途中の `#N`、区切りspaceなし、spaceが複数、全角記号、全角数字、`#0`、先頭0、整数overflow、文字数overflowには候補を返さない。初期実装で曖昧な表記を正規化または推測しない。
- 候補はevent名inputの編集可能な初期値として示し、「過去の末尾名から入れた候補であり、名前を変更しても同じ活動としてまとまる」とtextで説明する。候補の採用、編集、削除だけでは自動submitまたは画面遷移を起こさない。候補を作れない場合もerrorにはせず、理由を短く示した空のrequired inputからcontinuationを続けられるようにする。
- continuation pageから起点履歴と通常の単発作成へ戻れるlinkを置く。名前候補を消すこととseries continuation自体をやめることを同じ操作にしない。seriesへ追加しないで同じformを送るhidden切替は作らず、明示したcontinuationと通常作成を別のserver functionへ分ける。
- continuation createは起点public ID、planで受け取ったexpected tail public ID、通常と同じ検証済み `NewEventInput` を受け取る。serverはsession、起点の主催関係、現在のseries末尾、入力全体を毎回再検証し、clientのseries IDまたは推測結果を信頼しない。
- plan取得後に別のcontinuationがcommitされ、expected tailが現在の末尾でなくなった場合は409にする。古い候補名のまま二回分を保存せず、入力を残して「最新の続き情報を読み直す」操作を示す。再読込でも利用者が編集した名前を自動上書きしない。
- continuation createにsession cookieがない、壊れている、またはtransaction内で失効した場合はeventを保存しない。通常作成のanonymous fallbackはcontinuationへ適用せず、loginが必要な失敗として返す。
- randomなevent public IDと主催者capabilityは通常作成と同じ方法で生成する。capabilityはhashだけをDBへ保存し、生値は作成成功の一responseでだけbrowserへ返す。seriesはaccount sessionのread/grouping関係であり、主催者capabilityの代わりにしない。
- migrationで `event_series` と `event_series_members` を追加する。`event_series` は内部ID、owner account ID、表示名、作成時刻を持つ。表示名は最初の起点名から、厳密な連番を除ける場合はbase、除けない場合はtrim済み起点名を使い、別の必須入力を要求しない。
- `event_series_members` はseries内部ID、owner account ID、event public ID、0始まりのmembership positionを持つ。event public IDをprimary keyにして一event一seriesを、`(series_id, position)` のunique constraintで一series一順番を固定する。
- seriesの `(id, owner_account_id)` とeventの `(public_id, organizer_account_id)` をそれぞれ一意なparent keyにし、membershipから二つのcomposite foreign keyを張る。存在だけでなくseries ownerとevent organizerが同じaccountであることをschemaで固定する。
- account削除ではseriesとmembershipをcascade deleteする。既存方針どおりeventの `organizer_account_id` はNULLになり、public event、候補、response、decisionは保持する。event削除が将来導入された場合、そのmembershipだけはcascade deleteする。
- 初回continuationは一つの `BEGIN IMMEDIATE` transactionで、active session、起点の主催関係、現在のmembershipを再確認し、series、起点membership、new event、候補日時、new membershipをcommitする。既存seriesでは同じtransactionでcurrent tailを再確認し、checkedな次positionへ追加する。
- `BEGIN IMMEDIATE` とunique/composite foreign keyにより、同じ未所属eventから並行して作成してもseriesを二つへ分裂させない。競合は部分成功へせず、stale tailまたはconstraint errorとしてrollbackする。
- series membershipは名前から独立させる。候補名を `ベストユニゾン 夏回` へ編集しても同じseriesへ保存し、履歴groupは名前を再解析せずmembershipだけから構成する。これを表記揺れを後から扱う最初の余地とする。
- account履歴projectionは、主催した単発event、主催したseries、参加したeventを分ける。seriesは表示名とmember eventの既存最小履歴itemだけを返し、memberをposition降順で新しい回から辿れるようにする。owner accountのseriesに属するmembershipをscope条件で先に消さず、eventとowner関係をLEFT JOIN相当で検査地点まで運ぶ。空series、一memberだけのseries、消失event、owner不一致、重複positionはdata invariant違反として部分表示しない。
- 参加履歴はseries groupingを行わない。回答しただけのaccountへ、主催者が定義した活動関係を新しいprivate情報として広げない。
- 履歴のseries groupはnative `details` と `summary`、memberの `ul` で表し、summary内へlinkまたはbuttonをnestしない。memberには保存した元のevent名を表示し、名前をgroup表示名へ置き換えない。
- UIはloading、guest、expired、missing、failure、候補あり、候補なし、送信中、validation、stale plan、作成成功を区別する。route public IDをreactive dependencyとkeyへ含め、別eventへ遷移したとき古いprivate planを表示しない。
- 320 CSS pxでは一列にし、長い起点名、series表示名、説明、編集したevent名を折り返す。event名input、候補日時、通常作成へのlink、submitをkeyboardで操作でき、非同期成功後は作成成功見出し、失敗後は関係する案内へfocusする。
- 初期実装では、既存eventを後から任意のseriesへ付け替える操作、seriesのrename・分割・統合、複数owner、participant向けgroup、類似度判定、AI分類、保存済み命名ruleを追加しない。

## rejected options

### event名の一致または類似度から自動でseriesを作る

既存履歴を操作なしでまとめられる。
しかし、表記揺れで分裂し、偶然似た別活動を誤結合する。修正UIもないため採用しない。

### すべての新規作成formへseries選択を置く

どのeventでも最初から分類できる。
しかし、anonymous-firstの短いflowに、単発利用者には不要な仕事を増やす。過去の主催履歴から明示したときだけ扱う。

### 履歴または作成画面を開いた時点で `#N` を探して提案する

少ない操作で候補を見せられる。
しかし、活動を続けたいという意思がない `飲み会 #17` にも `#18` を自動提案する。明示的なcontinuation開始を意味のgateにする。

### 全角記号、日付、括弧、ローマ数字も正規化して次回名を推測する

多くの表記に対応できる。
しかし、初期実装での誤提案と説明不能な規則を増やす。parserは厳密な ` #N` だけにする。

### 命名ruleまたはregular expressionをseriesへ保存する

次回以降の提案を安定させられる。
しかし、利用者が編集した名前とruleのどちらを正とするか、新しい管理UIが必要になる。毎回、現在の末尾名から保守的に計算する。

### 通常のcreate inputへnullableなseries IDを追加する

endpointとformを共用しやすい。
しかし、anonymous APIへaccount-privateなidentityと認可分岐を持ち込み、client指定の内部IDを検査する範囲が増える。privateなcontinuation createを分ける。

### eventを通常作成した後、別requestでseriesへ追加する

既存のcreate repositoryを変更せずに済む。
しかし、eventだけ作成されてgroupingに失敗する部分成功を作る。event aggregateとmembershipを一transactionへ置く。

### session失効時は単発のanonymous eventとして保存する

入力したevent自体は失わずに済む。
しかし、利用者が同じ活動へまとまったと誤認する。continuationではrollbackし、入力をbrowserへ残す。

### series内部IDをclientへ返して直接指定させる

APIとclient側groupingを単純にできる。
しかし、account scopeをclient入力へ移し、IDORの検査面を増やす。起点eventとserver-side membershipからseriesを解決する。

### 参加履歴も主催者のseriesでまとめる

同じ活動を参加者も辿りやすい。
しかし、回答関係だけのaccountへ、別eventとの関連を新たに開示する。初期実装は明示したownerの主催履歴だけに留める。

## consequences

- 通常の匿名作成と回答にはfield、request、分類処理が増えず、loginなしの最短flowを維持できる。
- seriesは主催者の明示操作でだけ生まれ、名前を編集してもmembershipが保たれるため、誤った自動分類と表記揺れを別の問題として扱える。
- strict parserは説明しやすくtestしやすい一方、`第2回`、`# 2`、全角数字、日付suffixなど妥当な規則を提案できない。これは誤提案を減らすために飲み込むfalse negativeである。
- 利用者が明示的に `飲み会 #17` のcontinuationを開始すれば `#18` は候補になり得る。活動の意味を文字列から判定せず、明示操作と自由編集を意味の根拠にするための残余リスクである。
- planとcreateの間に別の次回が作られると409になり、再確認が一手増える。その代わり、同じ末尾から重複した連番を黙って作らない。
- composite foreign keyとunique indexが増え、migrationとrepositoryは複雑になる。その代わり、一event一series、owner一致、series内順番をapplication codeだけに頼らず固定できる。
- account削除後もeventはpublic-by-linkで残るが、privateなseries groupingは失われる。account履歴の追加価値を削除後まで別ownerなしで保持しない判断である。
- series表示は主催履歴だけで、参加者は同じ活動を横断できない。新しいcross-event開示を避けるための初期制約であり、participant向け価値は別Storyで認可を決める必要がある。
- series rename、既存eventの後付け、分割、統合をまだ行えない。membership schemaは名前から独立しているため将来拡張できるが、その操作と監査は新しいStoryとADRが必要である。
- private routeと二つのserver function、追加migration、grouped history projectionによりtest面は広がる。名前parser、認可、stale tail、transaction rollback、account delete、SSR privacy、responsiveを独立して固定する必要がある。
