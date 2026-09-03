# ADR 0009: 一回答へ任意のひとことを一件だけ追記する

Status: accepted

Date: 2026-09-02

## context

Product Story 3では、回答者が名前と全候補の都合を保存したあと、任意で短いひとことを返す。
ひとことは情報収集の必須欄ではなく、「調整ありがとう！」「楽しみ！」「肉！」のような発話そのものを歓迎する機能である。

回答の送信requestへひとことを混ぜると、回答完了までに入力を促す一手が増え、保存失敗の境界も一つになる。
一方、回答後の別requestにすると、ログインしていない回答者が、自分の直前の回答だけへ安全かつ重複なく追記できる認可が必要になる。

Story 2で生成した回答capabilityは、同じ回答の再送と回答直後の追加操作を認可するため、親componentのmemoryにだけ残っている。
この秘密を公開URL、DOM、永続storageへ広げず、一回答につき0件または1件のひとことを保存する境界を決める。

## decision

- `responses` にnullableな `respondent_comment` 列を追加する。`NULL` はひとことを送っていない状態、非NULLはその回答へ保存された一件のひとことを表す。
- 本文はRustのUnicode `trim()` で前後の空白を除き、内部の空白と改行は保持する。1文字以上500文字以内とし、NULは拒否する。500文字は既存の主催者のひとことと同じ上限である。
- DBにも、NULLまたは1文字以上500文字以内だけを許す `CHECK` を置く。Unicode空白の判定はserver validationを正とし、SQLiteの制約は迂回経路への追加防御とする。
- comment ID、独立timestamp、追加indexは持たない。初期MVPは一回答につき一件だけであり、ひとことをchatや編集履歴へ広げない。
- server functionは、公開イベントID、回答capability、本文だけを受け取る。内部response ID、回答者名、候補IDをclientから受け取らず、認可にも使わない。
- serverは回答capabilityをSHA-256 hashにし、公開イベントIDとhashの両方が一致する一回答だけを対象にする。生のcapabilityをDB、URL、HTML、DOM attribute、hidden input、server log、client log、`localStorage`、`sessionStorage`、API応答へ残さない。
- 有効な形だが回答に一致しないcapability、別eventとの組合せ、存在しない回答は、同じ404として扱う。入力の形や本文のvalidationは422、保存済みと異なる本文は409、予期しないDB失敗は500とする。回答APIと同じく、戻り値では `ServerFnError` を明示してHTTP statusを保つ。
- repositoryは短い `BEGIN IMMEDIATE` transactionで対象回答を読み、本文がNULLなら保存する。同じtrim済み本文の再送は既存の一件を成功として返し、異なる本文は元の発話を上書きせずconflictにする。
- 回答成功表示を先に置き、「ここまでで回答は完了」「このまま閉じてよい」と伝えたあと、独立した任意sectionでひとことを促す。
- 任意sectionには、即送信しない二つの例文button、自由入力textarea、送信button、「今回は送らない」buttonを置く。例文buttonはtextareaへ値を入れてfocusするだけにし、確認なしでは送信しない。
- ひとことを保存したとき、または「今回は送らない」を選んだときは、親componentが生の回答capabilityを破棄する。保存失敗時は本文とcapabilityを保持し、同じ内容を再試行できる。
- ひとことUIにはcapabilityそのものをpropsで渡さず、親componentが秘密を閉じ込めたcallbackだけを渡す。ひとこと入力を自動focusせず、まず回答完了見出しへfocusする。
- Story 3ではコメントの読取APIや一覧を公開projectionへ加えない。Story 4で主催者向けの読取認可と表示範囲を決めるまで、保存成功だけを回答者へ伝える。
- 本文はplain textとして保存し、後続表示でも通常のRSX text nodeとしてescapeする。MarkdownやHTMLとして解釈しない。

この境界なら、ひとことの失敗やskipは、すでに保存された回答を未完了へ戻さない。
また、同じ回答capabilityを最小期間だけ再利用し、新しいaccount、公開識別子、永続tokenを追加せずに匿名回答へ発話を結び付けられる。

参考資料：

- [SQLite: ALTER TABLE](https://www.sqlite.org/lang_altertable.html#altertabaddcol)
- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html#deferred_immediate_and_exclusive_transactions)
- [OWASP: Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html#data-to-exclude)
- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/)
- [Dioxus 0.7: Escaping RSX](https://dioxuslabs.com/learn/0.7/essentials/ui/escape/)

## rejected options

### 元の回答送信requestへひとことを含める

一つのtransactionで回答とひとことを保存できる。
しかし、任意入力が最短回答経路の中へ入り、ひとことの失敗で都合回答まで未完了に見えるため採用しない。

### `response_comments` を別tableにする

複数コメント、独立timestamp、編集履歴へ広げやすい。
現在は一回答につき0件または1件だけであり、ID、外部キー、JOINを増やしてchat化しやすい形を先回りする必要がないため採用しない。

### 回答者名または内部response IDで回答を指定する

単純なlookupで追記できる。
名前は本人確認ではなく同名回答を許しており、内部IDは推測・改変できるため、認可には使わない。

### capabilityをひとことcomponent、URL、hidden input、永続storageへ渡す

子componentやreload後の処理から参照しやすい。
意図しない描画、共有、log、XSSから読める保持範囲を広げるため、親componentのcallbackへ閉じ込める。

### ひとこと専用の第二capabilityまたはidempotency keyを発行する

回答再送とひとこと認可の役割を分けられる。
初期MVPの一回だけの追記は、既存の強い回答capabilityと保存済み本文の比較で識別できるため追加しない。

### 異なる本文の再送で無条件に上書きする

簡単な編集として振る舞える。
commit後に応答だけを失った利用者が本文を変えた場合、先に保存された発話を気付かず失う。Story 3は編集を要求していないため、409として元の本文を保つ。

### 一回答から複数のひとことを送れるようにする

追加の発話や会話を同じ画面で続けられる。
First Instructionの「グループチャットにしない」を越え、回答後の軽い一言という目的をぼかすため採用しない。

### 例文buttonを押した時点で送信する

一操作で短いひとことを送れる。
Story 3では編集を提供せず、誤操作を取り消せないため、textareaへ反映して本人が送信を確定する二段階にする。

## consequences

- 都合回答はひとことより先に独立してcommitされる。ひとことを送らない、または送信に失敗しても回答完了は変わらない。
- 一回答へ0件または1件のひとことを、追加JOINなしでStory 4・5のresponse projectionから読める。
- 同じ本文の再試行は重複せず成功し、異なる本文の再送や意図しない編集は元の発話を上書きしない。
- commit後の応答喪失中に本文を変えて再送すると409になる。この場合、変更後本文を保存したとは表示できず、先のひとことが保存済みである可能性を案内する必要がある。
- capabilityを保存成功またはskipで破棄するため、その後の編集、reload後の追記、別端末からの追記はできない。復旧や編集が必要になれば、認可と保持期間を改めて決める。
- 「今回は送らない」はDBへ記録しないため、意図的なskip、画面を閉じた状態、まだ入力していない状態を後から区別できない。Story 3にはその分析を要求しない。
- nullable列は初期要件を小さく保つ一方、複数発話、編集履歴、削除履歴が必要になれば別tableへのmigrationが必要になる。
- 本文は平文でSQLiteへ残る。Story 4では、誰がコメントを読めるかを主催者向け読取APIの認可と一緒に決める必要がある。
- 500文字と一回答一件で増加量を抑えるが、匿名endpointへの自動投稿やDB肥大化は残る。公開運用前にrate limitと保存上限を再検討する。
- `BEGIN IMMEDIATE` により同じ回答への競合を直列化する。ローカル単一instanceには適するが、水平分散時は別の排他設計が必要になる。
- 生のcapabilityはDOMや永続領域へ出ないが、認可credentialとしてHTTPS上のPOST bodyと実行中memoryには存在する。本番公開時はTLSを前提にする。
- Story 3ではコメントを表示しないため、保存した発話を主催者が読む価値はStory 4が完成して初めて利用者へ届く。
