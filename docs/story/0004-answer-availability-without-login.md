# Story 0004: 共有URLからログインせず都合を返す

Status: in progress

Date: 2026-09-02

Product sequence: Story 2

## context

Product Story 1で、主催者はイベントを作り、イベント名、任意のひとこと、候補日時を共有URLから見せられるようになった。
回答者はまだ候補を読むことしかできず、自分の都合を主催者へ返せない。

First Instructionの最短回答経路は、共有URLを開き、主催者の意図を読み、名前と各候補の○、△、×だけを送る。
ログイン、プロフィール、カレンダー連携、コメント入力を先に要求しない。

## definition of done

- 回答者が共有URLを開くと、イベント名、主催者のひとこと、全候補日時と回答フォームを同じ画面で確認できる。
- ログインしていない回答者が名前を入力し、各候補へ○、△、×のいずれかを選べる。
- ○は「行ける」、△は「条件次第・たぶん行ける」、×は「難しい」と画面上で理解できる。
- 名前が空、または未回答の候補がある場合は、原因が該当箇所と結び付いた日本語のerrorとして示される。
- 回答者名と全候補の都合がSQLiteへ一つのtransactionで保存される。
- 候補ID、イベントID、回答値を改変したrequestは、サーバー側の検証で保存されない。
- 保存後は回答完了が明確に示され、同じ送信操作による二重送信を防ぐ。
- 保存に失敗した場合は、名前と選択した○、△、×が画面に残り、そのまま再試行できる。
- 回答完了までに、ログイン、プロフィール作成、カレンダー連携、コメント入力を要求しない。
- 320px幅で横スクロールせず、候補ごとの選択と送信をキーボードだけでも完了できる。
- 利用者に見える受け入れテストと保存境界のtestを実装より先に追加し、期待した理由でREDになる。
- test、Clippy、format、Fullstack build、秘密情報検査が成功する。
- 8081の候補版を検証している間も、検証済みserverを別portで利用できる。

## to do

- [x] First InstructionのProduct Story 2と、Story 3以降との境界を確認する。
- [x] 匿名回答のaggregate、重複送信、入力上限、回答後に使う識別子をADRへ記録する。
- [x] 回答フォーム、入力検証、回答完了表示の失敗する受け入れテストを書く。
- [x] responseとavailabilityのmigration、transaction、改変request拒否の失敗するtestを書く。
  - `cargo test --test availability_response` は未実装のdomainとUIを解決できずREDになった。
  - `cargo test --test answer_repository --features server` は未実装の保存境界を解決できずREDになった。
  - responsive testは `.availability-options` がないためREDになった。
- [x] 共有画面へ名前と○、△、×の回答フォームを実装する。
- [x] 回答をserver functionから検証し、SQLiteへ保存する。
- [ ] 320pxとdesktopの実ブラウザーで、通常、validation、保存失敗、keyboard経路を確認する。
- [ ] 別browser contextとserver再起動後に、保存された回答をrepositoryから確認する。
- [x] 独立レビューを反映し、README、Story、対応表、検証記録、Surprise & Discoveryを更新する。

## concern

- 名前は本人確認ではない。同名の回答者と、他人の名前を入力する回答者を技術的には区別できない。
- 匿名の作成・回答APIには自動投稿やDB肥大化の余地がある。初期MVPでは一requestの上限を設けるが、公開運用ではrate limitを別途判断する必要がある。
- 候補を一つでも未回答のまま送れると、未回答と×の意味が曖昧になる。最短経路を保ちながら、全候補への明示選択を要求する。
- network応答をcommit後に失う場合、buttonの二重操作防止だけでは再送の重複を完全には防げない。再試行可能性と冪等性の境界をADRで決める必要がある。
- 回答後のひとこと、回答集計、回答の編集、主催者の日程決定は後続Storyの範囲である。
- public-by-linkではURLを知る人が回答できる。招待制または本人確認が必要な情報は扱わない。
