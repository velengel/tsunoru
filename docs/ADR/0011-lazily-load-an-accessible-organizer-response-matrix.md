# ADR 0011: 主催者用集計表を必要時だけ取得する

## context

Product Story 5では、回答サマリーだけで判断できない主催者が、回答者と候補日時を交差させた従来型の集計表へ戻れるようにする。
集計表は全回答者、全候補、全交点を保持するため、回答数をR、候補数をCとするとpayloadとDOMがO(R × C)になる。候補は最大20件だが、匿名回答数には上限がない。

回答者名と個別の○、△、×はprivate dataである。
Story 4と同じ主催者capabilityで認可し、生の値、hash、内部IDをURL、SSR HTML、応答へ出さない必要がある。
一方、サマリーの最短経路へ大きな行列を常時混ぜてはいけない。

表は二次元の対応関係そのものに意味がある。
W3C WAIは、上段と先頭列に見出しがある表へ `th` と `scope="col"`、`scope="row"` を使い、`caption` で表を識別する方法を示している。
WCAG 2.2のReflowはdata tableを二次元配置の例外に挙げる一方、例外は表の部分だけであり、周囲の内容までpage横scrollへ巻き込まないよう説明している。

Dioxus 0.7.10は `ontoggle` を扱えるが、portableな `ToggleData` に `details` のopen状態がない。
遅延取得の開始と閉じる操作を区別するにはweb専用DOMへのdowncastが必要になる。
一方、`use_action` は利用者が明示的に呼ぶまで実行せず、再呼出しやresetで進行中taskをcancelできる。

参考:

- [WAI: Tables with Two Headers](https://www.w3.org/WAI/tutorials/tables/two-headers/)
- [WAI: Caption & Summary](https://www.w3.org/WAI/tutorials/tables/caption-summary/)
- [WCAG 2.2 Understanding: Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html)
- [WHATWG HTML: details](https://html.spec.whatwg.org/multipage/interactive-elements.html#the-details-element)
- [Dioxus 0.7: Event Handlers](https://dioxuslabs.com/learn/0.7/essentials/basics/event_handlers/)
- [Dioxus 0.7.10: use_action](https://docs.rs/dioxus-hooks/0.7.10/dioxus_hooks/fn.use_action.html)
- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html)
- [SQLite: Isolation](https://www.sqlite.org/isolation.html)
- [SQLite: ORDER BY](https://www.sqlite.org/lang_select.html#orderby)

## decision

- 集計表は、主催者用サマリーrouteの中から必要時だけ開く補助表示にする。Story 4の初期summary payloadと公開event projectionへ個別回答を追加しない。
- private endpointを `POST /api/organizer/events/matrix` とする。既存の `OrganizerSummaryInput` を主催者private readの認可envelopeとして再利用し、同形の秘密入力型と検証処理を増やさない。
- 生のcapabilityはlocalStorageを読む非同期helper、または既存の復旧キーformの入力にだけ置く。Signal、projection、props、DOM、URL、logへ保持しない。64文字の小文字16進数として検証し、SHA-256 hashへ変換した後に破棄する。
- 不在event、誤ったcapability、別eventのcapabilityは同じ404にする。入力形式は422、DB失敗とaggregate不変条件違反はprivate内容を含まない500にする。成功と全error応答へ `Cache-Control: no-store` を付ける。
- repositoryはDEFERRED read transactionを開始し、最初のSELECTでevent IDとorganizer capability hashを照合してsnapshotを確立する。同じtransactionで候補、回答、全availabilityを読み、HTTP serializationより前にcommitする。
- 集計済みtableと列は作らない。未知のresponse IDやcandidate IDを持つ破損cellも検査前にJOINで落とさないため、availabilityはevent IDで直接取得する。そのlookupを全table scanにしない `response_availabilities(event_public_id)` indexだけを追加する。
- wire projectionはevent名、eventのタイムゾーン、作成順の候補日時vector、保存順の回答行vectorだけを持つ。各回答行は回答者名と、候補vectorの位置に対応する `Vec<Availability>` を持つ。
- candidate IDとresponse IDはrepository内部の再構成にだけ使い、projectionへ含めない。コメント、件数サマリー、判断補助ラベル、score、順位、決定状態も含めない。
- 候補は `position ASC`、回答は内部response IDの昇順で決定的に並べる。response ID順を「送信時刻順」とは呼ばない。availability queryの返却順には依存せず、内部IDから二次元領域へ配置する。
- 候補が1件以上あること、`回答数 × 候補数` をoverflowなく計算できること、全交点が一度ずつ存在すること、各保存値が○、△、×へ変換できることを確認する。欠損、重複、未知の交点があれば部分表を返さない。
- 回答0件は正常な200とし、候補vectorと空の回答行vectorを返す。UIでは空tableを描画せず、「まだ詳細回答はありません」と伝える。
- 同じ回答者名をまとめない。内部response IDが異なる回答は、表示名が同じでも別行として残す。
- 遅延取得のdisclosureはnative buttonで実装し、`aria-expanded` と `aria-controls` を同期する。Dioxusのhookは条件分岐より前のcomponent直下へ置き、`use_action` の戻り値をunitにして、大きなpayloadはcapabilityを含まない独立stateだけに置く。
- 初回の展開でだけPOSTを開始する。連打した再取得は前のtaskをcancelする。閉じたときはtableをDOMから外すが、同じsummaryを表示している間は取得済みprojectionを再利用してよい。
- summaryの更新または復旧が成功したら、進行中のmatrix taskをcancelし、matrix state、payload、DOMを破棄してdisclosureを閉じる。summary更新が失敗した場合は、表示中のsummaryとmatrixを保持する。
- matrix取得時にlocalStorageのcapabilityがない、または拒否された場合は、既存の復旧キーformを表示する。復旧成功はsummaryを再取得してcapabilityを保存した後、matrixを未取得へ戻す。利用者は改めて表を開く。
- 表はnative `table`、`caption`、列の `th scope="col"`、行の `th scope="row"` を使う。○、△、×は視覚記号と「行ける」「条件次第」「難しい」のtextを同じcellで伝える。interactive gridへ変えず、個々のcellをTab stopにしない。
- tableを `role="region"`、`tabindex="0"`、名前と操作説明を持つ横scroll containerへ入れる。page全体は320pxで横overflowさせず、二次元scrollをこの領域だけへ限定する。先頭の回答者列だけをstickyにし、長い名前は省略せず折り返す。縦scrollとsticky headerは追加しない。
- Story 5では、全回答を黙って切り捨てるLIMIT、pagination、virtualizationを入れない。score、推薦、候補の並べ替え、日程決定controlも追加しない。

## rejected options

### 回答サマリーAPIへ全行列を常時含める

表を開かない主催者にもO(R × C)のpayloadとbrowser memoryを負わせる。
デフォルトを軽くし、必要なら詳細へ進めるプロダクト原則に反するため採用しない。

### 公開event APIへ個別回答を追加する

共有URLを知る回答者全員へ、回答者名と個々の都合を公開してしまう。
主催者専用の情報境界を壊すため採用しない。

### `use_server_future`、`use_resource`、mount時のeffectで取得する

利用者が表を必要としなくてもrequestが始まる。SSR hydration payloadへprivate dataが入る境界も増えるため採用しない。

### native `details` の `ontoggle` から取得する

contentを既に持つStory 4のコメントには適するが、Dioxus 0.7.10のportable eventからopen状態を読めない。
web専用downcastを追加してまで遅延取得を結び付けず、状態を明示できるbuttonを使う。

### mobileだけ回答行をcardへ変換する

各候補を同じ横軸で比べる従来型集計表の情報関係が、画面幅によって変わる。
native tableの意味を保ち、局所scrollで全列へ到達させる。

### respondent nameを行の識別子にする

表示名は本人確認済みidentityではなく、同名回答もあり得る。deduplicateまたはmap keyにすると記録済み回答を失うため採用しない。

### `created_at` を回答順にする

SQLiteの既定値は同じ秒になり得る。決定的なtie-breakerを別に増やさず、内部response IDの昇順を使う。

### 一つの巨大JOINまたはSQLite JSON集約で返却形を作る

event名、候補、回答者名を各cellへ反復し、欠損と重複の検査、Rustのenum変換を読みにくくする。
候補、回答、cellを分けて読み、型付きvectorへ再構成する。

### `BEGIN IMMEDIATE` または集計済みmatrix tableを使う

読み取りが回答writerを不要に塞ぐか、write時の二重管理とdriftを生む。
現在値をDEFERRED read snapshotから計算する。

### LIMITで一部だけ返す、または最初からpaginationする

LIMITは「全回答を確認できる」という完了条件を黙って破る。
複数HTTP requestのpaginationは同一SQLite snapshotを維持せず、cursorと更新中表示の追加判断が必要になるため、このStoryでは採用しない。

### コメントと日程決定controlを表へ加える

コメントはStory 4のpreview、日程決定はStory 6の責務である。表の幅と判断の境界を壊すため含めない。

## consequences

- 閉じた状態の初期payloadとDOMは小さいままになる。一方、一度開けば全回答行をmaterializeするため、回答数が大きいeventではserver memory、JSON、WASM memory、描画時間が増える。
- 候補は最大20件だが、回答数は無制限である。実測で操作不能、応答遅延、memory圧迫が起きた場合は、回答受付上限またはhigh-water mark付きkeyset paginationを別ADRで判断する。
- summaryとmatrixは別request、別snapshotである。間に回答が届けばmatrixの方が新しい場合がある。matrix自身は一つのsnapshotで完全だが、先に見たsummaryと同時点とは保証しない。
- summary更新成功後にmatrixが閉じるため、詳細を見直すには再度buttonを押す必要がある。古い表を最新値として残さないための操作コストとして受け入れる。
- button disclosureはopen状態、accessible name、loading、error、retryをapplication側で管理する必要がある。native `details` より実装とtestが増える。
- 先頭列のsticky表示は横scroll中の対応関係を保つ一方、狭い画面の可視幅を使う。幅を制限し、文字を省略せず折り返す。
- static tableのcellはTab移動先にならない。支援技術のtable navigationと、keyboard focus可能なscroll領域を組み合わせる。実ブラウザーとVoiceOverでの確認が完了するまでは操作証拠をUNVERIFIEDとする。
- JavaScriptまたはWASM hydration前にはprivate matrixを取得できない。これはlocalStorageのcapabilityをcookieやSSRへ移さない結果である。
- 既存の復旧キーformを経由する場合、復旧後に表をもう一度開く一手が増える。localStorageが途中で失われた例外経路として受け入れる。
- 既存の `OrganizerSummaryInput` とstorage error名はStory 4由来で、利用範囲より狭い名前になる。同形の秘密型を重複させないことを優先し、一般名へのrenameは挙動変更と混ぜない。
- localStorageへ主催者capabilityを置く既存方針と、同origin XSSが主催者権限へ到達できるリスクは変わらない。
- event-scoped availability indexは完全性検査を保った読取を局所化する一方、回答の各cell insertへindex更新と保存領域を追加する。
