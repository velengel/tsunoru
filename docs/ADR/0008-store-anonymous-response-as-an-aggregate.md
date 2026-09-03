# ADR 0008: 匿名回答をaggregateとして保存し、回答capabilityで再送を冪等化する

Status: accepted

Date: 2026-09-02

## context

Product Story 2では、回答者名と、イベントにある全候補への○、△、×を一つの回答として保存する。
後続Storyは、この回答から候補別の人数、回答者別の集計表、回答後の任意のひとことを扱う。

名前は本人確認ではないため、同名の別人を技術的に区別できない。
また、送信buttonを一時的に無効にするだけでは、DBへのcommit後にnetwork応答だけを失った利用者が再試行したときの重複回答を防げない。

ログインを加えず、改変requestを一回答へ混ぜず、同じ内容の再試行だけを安全に成功へできる保存境界が必要である。

## decision

- 一つの回答を、回答者名とeventへの参照を持つ `responses` と、候補ごとの都合を持つ `response_availabilities` に分けて保存する。
- 都合は `available`、`maybe`、`unavailable` の三値だけをDBの `CHECK` 制約で受け付ける。
- 回答者名は前後の空白を除き、1文字以上100文字以内とする。名前は識別子にせず、同じeventへ同名で複数回答できる。
- requestは、そのeventに保存されている全候補をちょうど一度ずつ含めなければならない。欠落、重複、余分な候補、別eventの候補はサーバーで拒否する。
- `response_availabilities` は `event_public_id` も持ち、responseとcandidateへの複合外部キーによって、両者が同じeventに属することをDBでも保証する。このため、既存の `candidates` と新しい `responses` に `(id, event_public_id)` の一意制約を置く。
- ブラウザーは、最初の有効な送信直前にWeb Cryptoの `crypto.getRandomValues` で32 byteの乱数を作り、64桁のlowercase hexadecimalである回答capabilityとして送る。
- 生の回答capabilityは同じ内容を再試行している間と、回答直後にStory 3のひとことを追加する間だけcomponent stateで保持する。URL、HTML、server log、DB、`localStorage` には残さない。
- DBには回答capabilityのSHA-256 hashだけを保存し、その列を一意にする。内部の整数 `responses.id` はブラウザーへ公開しない。
- 新規保存は `BEGIN IMMEDIATE` の短いtransactionで行う。eventと全候補を確認し、responseと全availabilityを同じtransactionで保存する。
- 同じ回答capabilityで再送された場合は、保存済みのevent、trim済み回答者名、候補IDと都合の集合を正規化済みrequestと比較する。完全一致なら既存の一回答を成功として返し、異なる場合は409 conflictとして元の回答を変更しない。
- server functionはブラウザー側の型とvalidationを信頼せず、event、名前、capabilityの形、候補件数、候補集合を再検証する。戻り値のerror型はDioxusの汎用 `Result` aliasにせず、`ServerFnError` を明示する。これにより、型へdecodeできたrequestのvalidationはHTTP 422、未知のeventは404、同一capabilityの異なる内容は409、予期しない保存失敗は500として返す。未知の回答値など型へdecodeできないHTTP requestは、関数本体へ届かず保存前に拒否される。Dioxus 0.7のJSON decoderはこのdecode errorをHTTP 500として返すため、400系であるとは約束しない。
- Story 2では回答編集を実装しない。入力を変えた後の新しい送信は、新しい回答capabilityを使う別回答である。
- 回答が一件でも保存されたeventの候補日時は、初期MVPでは削除または差し替えない。候補編集を追加する場合は、既存回答と履歴を部分化させないmigration方針を先に決める。

この形なら、回答をJSONへ閉じずに後続の `JOIN` と `GROUP BY` で集計できる。
回答capabilityをブラウザーで先に作るため、最初のserver応答を失っても、生の秘密をDBへ保存せず同じlogical submissionを識別できる。

参考資料：

- [SQLite: Foreign Key Support](https://www.sqlite.org/foreignkeys.html)
- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html)
- [SQLite: UPSERT](https://www.sqlite.org/lang_upsert.html)
- [Web Cryptography Level 2: getRandomValues](https://www.w3.org/TR/webcrypto-2/#Crypto-method-getRandomValues)
- [OWASP: Session ID Entropy](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#session-id-entropy)
- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/)

## rejected options

### 一回答の都合をJSONとして保存する

一行でaggregateを保存できる。
しかし、候補への外部キーを張れず、後続の候補別集計と回答者別集計でJSON展開が必要になるため採用しない。

### 候補ごとの○、△、×の人数だけを保存する

サマリーは小さくなる。
回答者名、回答後のひとこと、従来型の集計表を復元できず、回答のtransaction境界も失うため採用しない。

### 回答者名をevent内で一意にする

名前を使った回答上書きを作りやすい。
名前は本人確認ではなく、同名の別人を一回答へまとめてしまうため採用しない。

### buttonの無効化だけで重複を防ぐ

同じ画面上の速い二度押しは防げる。
commit後の応答喪失、network再送、利用者の再試行を同じ送信だと判定できないため採用しない。

### 正規化したpayloadのhashを冪等性キーにする

追加の秘密を生成せずに済む。
同じ名前と同じ都合を持つ別人まで一回答へまとめてしまうため採用しない。

### 回答capabilityをserverで生成する

browser側の乱数生成処理が不要になる。
最初のserver応答を失うと、serverが返した生の値を再取得できず、同じ送信として再試行できないため採用しない。

### 冪等性IDと回答後の認可tokenを分ける

役割は明確に分かれる。
初期MVPでは二つのrandom値と一意列を管理する複雑さが増え、十分に強い一つのcapabilityで両方の境界を満たせるため見送る。

### responseとcandidateが同じeventかをrepositoryだけで検証する

schemaは小さくなる。
将来別の書き込み経路が増えたとき、一回答へ別eventの候補を混ぜる余地がDBに残る。`event_public_id` の重複を受け入れ、複合外部キーでも不変条件を守る。

### 回答capabilityをブラウザーの永続storageへ保存する

reload後にも回答を編集したり、ひとことを追加したりできる。
Story 2は回答編集を要求せず、秘密の保持期間とXSSから読める範囲を広げるため採用しない。

### server functionのerrorをDioxusの汎用 `Result` aliasへ包む

短く書け、既存のevent作成APIとも形がそろう。
しかし、Dioxus 0.7ではpayload内のapplication codeを保ったままHTTP statusが500になる。validation、not found、conflictをtransport上でも区別するため、回答APIでは明示的な `ServerFnError` を使う。

### 未知の回答値を受け取る独自のraw request型を置く

未知値を関数本体でvalidationし、HTTP 422へ統一できる。
一方、browserとserverで共有する三値の型に加えて文字列から変換する境界が増える。未知値は現状でも保存前に拒否され、Story 2の不変条件は守られるため、Dioxus 0.7のdecode statusだけを補正する独自層は置かない。

## consequences

- 回答者名と全候補の都合が、一つのtransactionで全部保存されるか、何も保存されないかになる。
- responseとcandidateのevent一致を、server validationとDB制約の両方で守れる。
- 同じ内容の再試行は回答を増やさず成功し、同じcapabilityによる意図しない上書きは409になる。
- 回答APIのHTTP statusが `ServerFnError` のcodeと一致する一方、frameworkのerror型とresponse生成規則へ依存する。Dioxusを更新するときは、この契約をHTTP境界でも再検証する必要がある。
- 未知の回答値は保存されないが、Dioxus 0.7のJSON decode errorはHTTP 500になる。公開APIとしてclient errorの分類が必要になれば、独自request型またはframework更新を検討する。
- 32 byteのrandom capabilityをhash化して保存するため、DBファイルだけから回答後の認可を再現しにくい。
- `response_availabilities` に `event_public_id` を重複保持し、複合一意indexと外部キーも必要になる。schemaとinsertは単純な二表構成より冗長になる。
- `BEGIN IMMEDIATE` により、SQLiteの回答writeは短時間でも直列化される。ローカル単一instanceの初期MVPには適するが、多数の同時回答や水平分散には別のDB設計が必要になる。
- 回答capabilityを失うと、reload後にその匿名回答へStory 3のひとことを追加できない。永続的な回答編集または復旧が必要になった場合は、保持方法を改めて決める。
- Story 3では、成功後も生きているcomponent stateからひとこと入力へ回答capabilityを明示的に渡す必要がある。その際もDOM、URL、log、永続storageへ値を出さないtestが必要になる。
- 一つのcapabilityが冪等性と回答後の認可を兼ねる。将来、tokenの失効、rotation、複数端末での編集が必要になれば、役割を分離するmigrationが必要になる。
- `response_availabilities` は候補削除に追随してcascadeするため、回答受付後に候補だけを削除すると、回答aggregateが部分化する。候補を不変とする初期MVPの間は問題にならないが、編集機能でこの制約を無視できない。
- 同名回答、自動投稿、回答総数の肥大化は防がない。公開運用ではrate limitと保存上限を別途判断する必要がある。
- 入力変更後に新しいcapabilityで送ると、先のcommit結果を利用者が確認できないまま別回答が増える可能性がある。Story 2の冪等性保証は、同じ内容を同じ画面から再試行する範囲である。
