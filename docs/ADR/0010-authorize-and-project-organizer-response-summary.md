# ADR 0010: 主催者capabilityで認可した回答サマリーをsnapshotとして返す

Status: accepted

Date: 2026-09-02

## context

イベント、候補、匿名回答、任意のひとことはSQLiteへ保存できるようになった。
Product Story 4では、主催者が候補ごとの集まりやすさを確認する。

公開event取得へ集計を足すと、共有URLを知る回答者にも回答数、回答者名、ひとことが漏れる。
一方、回答数、候補別集計、コメントを別々のconnectionと時点で読むと、同時回答の途中で互いに矛盾する表示を作り得る。
コメント全件や回答者別の都合を最初から返すと、payloadと画面が無制限に増え、後続の集計表まで先取りする。

## decision

- 主催者用画面は、共有回答画面と分けた `/events/{public_id}/summary` とする。このURL自体は秘密として扱わない。
- 作成成功画面から主催者用画面へ進める。主催者用画面は、同じevent用の `tsunoru.organizer.{public_id}` をhydration後にlocalStorageから読む。
- privateなサマリーをSSRのloaderや `use_server_future` で取得しない。localStorageはserverから読めず、server futureの結果はSSR HTMLへserializeされ得るため、clientの `use_effect` から認可済みPOSTを呼ぶ。
- APIは `POST /api/organizer/events/summary` とし、`event_public_id` と生の主催者capabilityをJSON bodyで受け取るserver functionとする。query、path、fragment、hidden inputへcapabilityを入れない。
- serverはpublic IDと64桁lowercase hexadecimalのcapabilityを再検証し、capabilityをSHA-256化する。`event_public_id + organizer_capability_hash` の組で毎回認可する。
- event不在、誤ったcapability、別eventのcapabilityは同じnot-found結果とする。生のcapability、hash、内部response IDをAPI応答とserver logへ出さない。
- capabilityを持つrequest型は、生の値を出す自動 `Debug` を実装せず、必要なdebug表示では値をredactする。private API応答には `Cache-Control: no-store` を付ける。
- localStorageにcapabilityがない、または保存値で認可できない場合は、集計を描画せず、主催者用の復旧キーを `type=password` の明示入力から受け取る。成功した値だけを同じlocalStorage keyへ再保存し、入力stateから捨てる。
- 復旧キーの保存に再び失敗しても、取得済みサマリーは表示する。次の再読込では再入力が必要であることを伝え、生の値を画面へ再表示しない。
- SQLite schemaと集計済みtableは追加しない。既存の正規化済み回答から都度集計する。
- repositoryはDEFERREDのread transactionを開始し、最初のSELECTで主催者認可とsnapshotを確立する。同じtransactionで回答件数、候補別集計、コメント件数とpreviewを読む。
- 候補別集計はcandidatesを起点に `LEFT JOIN` し、回答0件でも全候補を作成順で返す。各候補で○、△、×の合計が回答件数と一致しなければ、データ不変条件違反としてサマリーを返さない。
- 回答件数は匿名のresponse aggregate数であり、本人確認済み人数ではない。UIでは「N人」ではなく「N件の回答」と表現する。
- サマリーprojectionはevent名、主催者のひとこと、タイムゾーン、回答件数、候補日時と三値の件数、コメント総数、コメントpreviewだけを持つ。コメントのない回答者一覧と、回答者別availabilityを含めない。
- コメントpreviewは回答者名とplain textのひとことを最大3件返す。コメント専用時刻がないため、response IDの降順を決定的な選択順に使い、送信時刻順とは呼ばない。総件数と表示上限をUIで明示する。
- 判断補助ラベルはSQLの日本語文字列ではなく、集計結果からRustのenumとして導出する。優先順位は、回答が1件以上ある場合に限り、全回答が○、×が0件、×が1件、○が全候補中で単独最多、の順とする。それ以外と最多同数にはラベルを付けない。
- ラベルは「回答した全員が○です」「△を含めると、回答した全員が参加できそうです」「×が1件あります」「○が最も多い候補です」と表示する。score、順位、勝者色、候補の並べ替え、自動決定は行わない。
- 更新は自動pollingせず、「最新の回答を読み込む」という明示操作で同じ認可とsnapshot取得をやり直す。通信失敗では保存済みcapabilityを削除しない。
- コメントはnative `details` と `summary` で畳み、候補カードを初期表示の主役にする。320pxでは候補一列、desktopでは二列、カード内の○、△、×は意味と件数を伴う三列とする。
- Story 4では回答者×候補日時のtable、回答者別availability、日程決定controlを実装しない。

参考資料：

- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/)
- [Dioxus 0.7: Fullstack SSR](https://dioxuslabs.com/learn/0.7/essentials/fullstack/ssr/)
- [SQLite: Transaction](https://www.sqlite.org/lang_transaction.html)
- [SQLite: Isolation](https://www.sqlite.org/isolation.html)
- [WHATWG HTML: Web storage](https://html.spec.whatwg.org/multipage/webstorage.html)
- [WHATWG HTML: The details and summary elements](https://html.spec.whatwg.org/dev/interactive-elements.html#the-details-element)
- [W3C: Understanding Reflow](https://www.w3.org/WAI/WCAG21/Understanding/reflow)
- [W3C: Understanding Status Messages](https://www.w3.org/WAI/WCAG22/Understanding/status-messages)

## rejected options

### 公開event取得へ回答サマリーを追加する

共有画面と一回のGETだけで表示できる。
しかし、public-by-linkの回答者へ集計、回答者名、ひとことを公開し、主催者専用というStoryの境界を失うため却下する。

### 共有回答画面へ主催者サマリーを混在させる

routeを増やさず、capabilityがあるbrowserだけ条件表示できる。
しかし、回答者の最短経路と主催者の判断画面が同じlayoutと状態機械を共有し、誤表示時の影響も大きくなるため分離する。

### capabilityをURLへ入れる

SSRで直ちに認可し、別端末へ管理linkを渡しやすい。
しかし、履歴、log、referrer、誤共有へ主催者権限を残すため、query、path、fragmentのいずれにも入れない。

### 主催者サマリーをSSRする

最初のHTMLから内容を表示できる。
しかし、serverはlocalStorageを読めず、capabilityをcookieまたはURLへ移す追加判断が必要になる。private dataをhydration payloadへ含める境界も増えるため採用しない。

### 集計済みtableを更新する

読み取りqueryを短くできる。
しかし、回答transactionとの二重書き込み、修復、driftを増やす。初期MVPの規模では既存rowの都度集計で十分なため追加しない。

### 複数queryをtransactionなしで実行する

実装は少し短い。
しかし、回答件数、候補別集計、コメントが異なるcommit時点を読む可能性があるため却下する。

### `BEGIN IMMEDIATE` で集計を読む

write transactionと同じ慣れた境界を再利用できる。
しかし、読み取りだけの画面が回答の書き込みを不必要に競合させる。DEFERRED read transactionのsnapshotで十分なため使わない。

### コメント全件または回答者別availabilityを返す

一回の応答で詳細をすべて見られる。
しかし、匿名回答数に上限がなくpayloadと画面の高さが増え続け、Story 5の集計表まで先取りする。Story 4では総数と最大3件のコメントpreviewに限る。

### score、順位、推奨候補を返す

一つの候補を強く目立たせられる。
しかし、○、△、×の重みは主催者の意図によって変わり、TSUNORUが最終判断を奪うため、事実を一段だけ解釈するラベルに留める。

## consequences

- 共有URLを知るだけでは、回答数、候補別集計、回答者名、ひとことを読めない。
- 主催者はログインせず回答サマリーを確認でき、作成時に手動保存した復旧キーを初めて実際の権限回復へ使える。
- private dataはclient hydration後に初めて取得するため、主催者画面はJavaScriptが無効、またはWASM hydration前にはサマリーを表示できない。
- `no-store` により通常のHTTP cacheへprivate responseを残さない一方、実行中のbrowser memoryとHTTPS request bodyには一時的に生のcapabilityが存在する。
- 専用routeを覚えていなくても作成成功画面から進めるが、その画面を離れた後にURLを失うと、共有URLから自動的には主催者画面へ移らない。
- localStorageの保存拒否、削除、同一originのXSSという既存の主催者capabilityリスクを引き続き受け入れる。
- read transactionにより一回の表示は一つのsnapshotに揃う。別connectionで後からcommitした回答は、明示的な再読込まで表示されない。
- 集計queryは回答数に応じてrowを読む。初期MVPでは受け入れるが、計測で必要になればindexまたはprojectionを後続ADRで判断する。
- response aggregate数は実人数ではない。同名、別名、同一人物の複数送信をまとめず、それを正確に伝える必要がある。
- コメントpreviewは最大3件なので、全コメントをこの画面だけでは読めない。画面の軽さと引き換えに受け入れ、詳細表示が必要になった時点で別Storyにする。
- コメント送信時刻を保存していないため、previewの選択順はresponse IDに依存する。真の「最新コメント」とは表現しない。
- 判断補助ラベルは候補を選ばず、同数最多では沈黙する。主催者の判断材料を絞りすぎる場合は、Story 5の集計表で補う。
- native disclosureはkeyboard semanticsをbrowserへ委ねられる一方、browser間のmarker appearance差を受け入れる。
