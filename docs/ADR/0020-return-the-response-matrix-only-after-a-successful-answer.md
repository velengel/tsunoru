# ADR 0020: 回答成功responseでだけみんなの回答を返す

## context

回答者名と候補ごとの○、△、×は、これまで主催者capabilityでだけ読めるprivateな集計表だった。
利用者は、自分の回答を送った後にみんなの回答を確認したい。

共有URLを開いただけで一覧を返すと、回答していない閲覧者にも個別回答を広げる。
別の読取endpointをresponse capabilityで認可する方法もあるが、URL再読込後の復旧、secretの保持期間、comment送信後の破棄を新たに決める必要がある。

回答保存は既にresponse capabilityでidempotentである。
初回保存または同じpayloadのretryをcommitした後、そのcapabilityが同じeventのresponseに結び付くことをread snapshotで確認し、成功payloadへ完全な一覧を含められる。

参考:

- [OWASP: Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html)
- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html)
- [SQLite: Isolation](https://www.sqlite.org/isolation.html)
- [WAI: Tables with Two Headers](https://www.w3.org/WAI/tutorials/tables/two-headers/)
- [WCAG 2.2: Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html)

## decision

- `POST /api/answers/submit` の成功型をunitから回答一覧へ変更する。初回保存と同一payloadのretryは、どちらも一覧を返す。
- response capabilityは従来どおりrequest bodyにだけ含める。serverは保存時と同じ正規化済みevent public ID、capability hashを使い、response rowの存在を再認可する。
- response保存の `BEGIN IMMEDIATE` は一覧読取前にcommitする。一覧は別のDEFERRED read transactionで、一つのsnapshotからevent、候補、全response、全cellを再構成する。
- 一覧のwire型は主催者用matrixと同じ最小projectionを共有する。event名、timezone、候補日時、回答者名、○・△・×だけを持ち、response ID、candidate ID、capability、hash、account、commentを含めない。
- participant readはevent public IDとresponse capability hashの両方が同じresponse rowへ一致した場合だけ認可する。別event、別capability、不在responseは同じnot-foundとして扱う。
- 欠損、重複、未知のcell、候補0件はdata invariant違反とし、部分一覧を返さない。既存のmatrix再構成処理を共有する。
- commit後のmatrix読取失敗はgenericな500とする。browserは入力と同じcapabilityを保持するため、同一payloadを再送すれば二重回答を作らず復旧できる。
- clientは成功payloadをcomponent stateへ保持し、回答完了の直後に一覧を表示する。任意commentの成功、skip、失敗で一覧を破棄しない。
- public event GET、初期SSR、回答formの表示だけでは一覧を取得しない。回答一覧専用のpublic endpoint、query parameter、cookie、localStorageを追加しない。
- 一覧は既存のaccessible table表現を再利用する。participant向けheadingを「みんなの回答」とし、横scrollをtable領域だけに閉じる。

## rejected options

### public event projectionへ全回答を含める

共有URLを知るだけで回答者名と個別回答を読める。利用者が求めた「送った後」より可視範囲が広いため却下する。

### 回答成功後にpublic GETで一覧を取得する

GETをpublicにすれば閲覧者全員へ広がり、secretを付ければURL、header、browser storageの新しい保持判断が必要になる。成功POSTへ含める方が境界が小さい。

### comment送信後に一覧を取得する

任意commentを一覧閲覧の条件にしてしまい、「回答はここまでで完了」という既存方針を壊すため却下する。

### response保存transaction内で一覧を組み立てる

全response×候補の読取中もSQLite writerを保持し、別の回答を不必要に待たせる。commit後のread snapshotへ分ける。

### 自分の回答行だけを返す

保存確認にはなるが、みんなの都合を見たい要求を満たさないため却下する。

### comment本文も一覧へ含める

commentは回答成功後の別操作であり、最初の一覧snapshotにはまだ存在しない場合がある。表示更新とprivacy範囲を増やさないため除外する。

## consequences

- 回答した人は、その場で全回答を確認できる。共有URLを開いただけの人には従来の公開境界を保つ。
- 成功responseの大きさとDOMは回答数×候補数に比例する。候補は20件までだが回答数は無制限である。
- response capabilityは一覧を返すserver内認可にも使うが、生値の保存場所と寿命は増えない。
- 保存commit後にreadが失敗すると、HTTP上は失敗でも回答は保存済みになり得る。idempotent retryが復旧手段となるため、clientは失敗時にcapabilityと入力を保持する必要がある。
- participantとorganizerは同じmatrix projectionと再構成処理を使うが、認可入口は別である。型名は認可主体を表さない中立名へ寄せる。
- 回答後一覧はそのresponse時点のsnapshotであり、後から別の人が回答しても自動更新しない。最新化操作は今回追加しない。
