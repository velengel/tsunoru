# ADR 0012: 最初に明示された日程決定を確定結果として保持する

## context

Product Story 6では、主催者が回答サマリーと集計表を判断材料にし、候補日時を一つ選んで確定する。
systemの推薦や多数決が自動で決めるのではなく、主催者の明示操作を保存する必要がある。

通信失敗後の再試行と、別tabからほぼ同時に行われる異なる候補の確定は、外形だけでは似ている。
前者は安全に成功へ戻し、後者は黙って上書きしない必要がある。
さらにProduct Story 7で決定日時を公開し、iCalendarとして持ち帰れるようにすると、後からの無通知な変更は参加者のcalendarを古くする。

SQLiteは同時に一つのwrite transactionだけを許し、`BEGIN IMMEDIATE` はtransaction開始時にwriterを確保する。
またPRIMARY KEYとUNIQUE constraintは一event一決定をdatabaseでも守れる。
候補とeventの組はavailability responseの複合foreign key用に追加した既存のunique indexを持つため、同じ親キーを再利用して別eventの候補を保存できないようにできる。

参考:

- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html)
- [SQLite: CREATE TABLE](https://www.sqlite.org/lang_createtable.html)
- [SQLite: Foreign Key Support](https://www.sqlite.org/foreignkeys.html)
- [SQLite: ON CONFLICT](https://www.sqlite.org/lang_conflict.html)
- [SQLite: Date And Time Functions](https://www.sqlite.org/lang_datefunc.html)

## decision

- MVPの日程決定は、一eventにつき最初にcommitされた一件を確定結果とする。後から別候補へ変更する操作はStory 6へ含めない。
- 同じ主催者が同じcandidateを再送した場合は、通信再試行として既存の決定を返し成功させる。異なるcandidateを送った場合は409 conflictとし、最後の書き込みで上書きしない。
- 二つの同時requestを直列化するため、repositoryは `BEGIN IMMEDIATE` を使う。transaction内で主催者認可、candidate所属、既存決定を確認し、新規決定をinsertしてからcommitする。
- 保存tableを `event_decisions` とし、`event_public_id TEXT PRIMARY KEY NOT NULL`、`candidate_id INTEGER NOT NULL`、`decided_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP` を持つ。
- `event_public_id` はeventへ、`(candidate_id, event_public_id)` はcandidateの既存unique keyへforeign keyで結ぶ。event削除時だけdecisionもcascadeし、決定済みcandidate単独の削除はrestrictする。
- `decided_at` はdatabaseが記録したUTCの確定時刻であり、開催日時ではない。監査と将来の競合説明のため保存するが、Story 6のbrowser projectionへは返さない。
- requestはevent public ID、candidate ID、生の主催者capabilityを持つ専用型にする。custom `Debug` でcapabilityを伏せ、64文字の小文字16進数を検証してSHA-256 hashへ変換した後に生値を破棄する。
- private endpointを `POST /api/organizer/events/decision` とする。関数へ到達した成功と全application errorへ `Cache-Control: no-store` を付ける。Dioxusが関数前に返すwire decode errorの境界は[ADR 0013](0013-keep-typed-dioxus-decode-errors-outside-the-application-contract.md)で追補する。
- event不在、誤ったcapability、別eventのcapabilityは同じ404にする。入力形式と非正candidate IDは422、認可済みeventに属さないcandidateは422、異なる既存決定は409、DB失敗はprivate内容を含まない500にする。認可をcandidate照合より先に行う。
- server responseはcandidate ID、local date、local timeだけを持つ主催者用決定projectionとする。capability、hash、`decided_at`、回答、コメントを含めない。
- `OrganizerEventSummary` は同じread snapshotから現在の任意の決定projectionも返す。決定を知るためだけの追加private requestをmount時に増やさない。
- UIはnative `fieldset` とradioで候補を一つ選ばせる。初期選択は空にし、判断補助ラベルや件数から事前選択しない。
- 選択した日時をform内で再表示し、別の明示的なsubmit buttonで「この日時に確定する」を実行する。modalや二重の確認dialogは追加しない。
- 保存中は重複submitを止める。失敗時は表示中のsummary、matrix、radio選択を保持して再試行できるようにする。
- 成功時は決定projectionをsummary stateへ反映し、選択formを確定結果へ置き換える。409を受けた場合はsummaryを再取得し、別tabで先に確定された結果を表示する。
- 回答0件でも確定を許す。回答数は主催者の判断材料であり、日程決定の認可条件にしない。
- 回答者用共有画面への決定表示、共有control、iCalendar、calendar account連携はStory 7まで追加しない。

## rejected options

### 異なる候補の再送で既存決定を更新する

単純だが、別tabの競合や再試行が後勝ちで確定結果を変える。
Story 7で既に持ち帰ったcalendarも黙って古くなるため採用しない。

### `INSERT OR REPLACE` またはupsertを使う

SQLiteのREPLACEは既存rowを削除してinsertする動作であり、最初の決定を保持する意味を壊す。
競合を成功に見せず、同一candidateかをapplicationで明示的に比較する。

### `events` tableへnullable candidate IDを追加する

event本体の一部としては自然だが、SQLiteの `ALTER TABLE ADD COLUMN` でcomposite foreign keyと決定時刻を一まとまりに追加できない。
独立tableの一rowを決定aggregateとする。

### transaction開始前に認可と既存決定を読む

確認後からinsertまでに別requestが決定できる。
認可、candidate所属、競合判断、insertを一つのwrite transactionへ置く。

### DEFERRED transactionからwriteへ昇格する

二つのreaderが同じ未決定状態を見た後にwriteへ進むと、一方がbusyになり得る。
小さな一決定writeでは開始時にwriterを確保し、結果を決定的にする。

### 回答件数または最多候補から自動確定する

○、△、×は参加可能性の情報であり、会場、目的、重要人物など保存していない判断を代替できない。
First Instructionどおり最後のハンドルを主催者へ残す。

### 判断補助で最上位のcandidateを初期選択する

submitだけでsystemの提案を追認しやすくなり、明示選択の意味が弱くなる。
radioは未選択から始める。

### 確定前にmodalで再確認する

誤操作対策になる一方、native formで選択とsubmitを分けた上にもう一段操作を増やす。
不可逆性は文言と選択中日時で示し、MVPでは二段階のform操作に留める。

### 回答が一件届くまで確定を禁止する

口頭調整済み、締切、会場都合など、回答数以外の理由で主催者が決める場合を妨げる。
未回答表示は残すが保存条件にしない。

### `decided_at` をbrowserから受け取る

端末時計を信頼する必要がなく、改変可能な値が増える。
databaseの `CURRENT_TIMESTAMP` をUTCの記録時刻として使う。

### 決定projectionを公開event APIへすぐ追加する

Story 6のprivateな確定操作と、Story 7の全員向け結果表示・共有・calendarを同時に広げる。
縦の境界を保つため、このStoryでは主催者画面だけに反映する。

## consequences

- 一度確定したeventは、MVPのUIとAPIから別候補へ変更できない。誤確定時は新しいeventを作る必要がある。
- 将来変更を許すなら、revision、変更理由、参加者への通知、古いcalendarの扱いを別ADRで決め、単純UPDATEにしない必要がある。
- `BEGIN IMMEDIATE` は競合を決定的にする一方、短時間でも他のwriteを待たせる。busy timeout内に認可、二つの小さなSELECT、insertだけを終え、network処理をtransactionへ入れない。
- databaseのPRIMARY KEYとcomposite foreign keyにより、一event一決定とcandidate所属をapplication bugだけに依存せず守れる。
- candidateの複合unique indexはmigration 0002で既に導入済みであり、重複するindexを追加しない。日程決定tableは同じschema上の親キーを再利用する。
- 同一candidateの再試行は成功するため、利用者は応答を受け取れなかった場合にも同じ選択を安全に送り直せる。
- 異なるcandidateの競合は409になり、UIはprivate summaryを再取得する追加requestが必要になる。
- summary型へ任意のdecision fieldが増えるため、既存のfixtureとserialization testを更新する必要がある。未確定eventでは `null` となり、既存の匿名作成・回答経路は変わらない。
- 決定tableは候補日時のコピーを持たないため、候補の正本は一つに保たれる。一方、将来candidate編集を導入する前に、決定済みcandidateをどう扱うか決める必要がある。
- Story 6完了時点では回答者は決定結果をまだ見られない。この短い非対称は、公開表示と正しいiCalendarを一まとまりで実装するStory 7までの開発境界として受け入れる。
