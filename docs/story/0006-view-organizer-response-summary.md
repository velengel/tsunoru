# Story 0006: 主催者が回答サマリーを見る

Status: in progress

Date: 2026-09-02

Product sequence: Story 4

## context

Product Story 2と3で、回答者は共有URLから全候補の○、△、×と任意のひとことを返せるようになった。
回答はSQLiteへ残るが、主催者が画面から確認する手段はまだない。

TSUNORUは多数決で日程を決める仕組みではない。
主催者が候補ごとの事実を短時間でつかみ、自分で決めるための材料として、回答数、○、△、×の件数、控えめな判断補助ラベル、ひとことを見せる。
回答者と候補を交差させる集計表や日程決定操作は、後続Storyへ分ける。

## definition of done

- ログインしていない主催者が、イベント作成時に保存した主催者capabilityを使って専用URLから回答サマリーを開ける。
- 主催者capabilityがない、形式が不正、別eventの値である場合は、回答数、候補別集計、回答者名、ひとことを一切返さない。
- localStorageへ主催者capabilityを保存できなかった主催者が、作成時に手動保存した復旧キーを入力して同じサマリーを開ける。
- サマリーは、回答aggregateの件数と、作成順を保った全候補の○、△、×件数を表示する。
- 回答が0件でも全候補を0、0、0で表示し、「全員が参加できる」のような空集合への誤った説明を出さない。
- 判断補助ラベルは、全員が○、△を含めれば全員参加できそう、×が1件、○が単独最多、の事実だけを優先順に一つ示す。
- 候補をscoreで順位付けせず、並べ替えず、「おすすめ」「最適」「この日に決める」と表示しない。
- 回答者のひとことは件数を示し、初回payloadと画面の高さを無制限に増やさない最大3件のpreviewとして表示する。
- ひとことpreviewには回答者名を添えるが、コメントのない回答者一覧や回答者ごとの都合は表示しない。
- 集計とひとことは同じSQLite read transactionのsnapshotから取得し、各候補の○、△、×合計が回答件数と一致しない場合は表示しない。
- 生の主催者capabilityとhash、内部response IDをURL、SSR HTML、server log、API応答へ出さない。
- 通信失敗時は既存の主催者capabilityを失わず、再試行できる。自動更新はせず、明示的な再読込を用意する。
- 320px幅では候補カードを一列、広い画面では二列にし、横スクロールせず全情報へ到達できる。
- loading、認可失敗、復旧キー入力、回答0件、通常表示、再試行をキーボードと支援技術から判別できる。
- Story 5の回答者×候補日時のtableと、Story 6の日程決定controlを先回りして実装しない。
- 利用者に見える受け入れtestと、認可・集計・snapshot境界のtestを実装より先に追加し、期待した理由でREDになる。
- test、Clippy、format、Fullstack build、秘密情報検査が成功する。
- 8081の候補版を検証している間も、検証済みserverを別portで利用できる。

## to do

- [x] First InstructionのProduct Story 4と、Story 3・5・6の境界を確認する。
- [x] 主催者専用route、capability認可、復旧キー、snapshot、判断補助ラベル、コメント上限をADRへ記録する。
- [x] サマリー、0件、控えめなラベル、コメント、loading、失敗、復旧の失敗する受け入れtestを書く。
- [x] 認可、候補順、○△×集計、コメント上限、snapshot不変条件の失敗するrepository testを書く。
- [x] 主催者専用のPOST server functionとSQLite read transactionを実装する。
- [x] 主催者専用route、復旧キー入力、候補カード、コメントdisclosure、再読込を実装する。
- [ ] 320pxとdesktopの実ブラウザーで、0件、複数回答、長いひとこと、keyboard、通信失敗、復旧を確認する。
- [ ] 回答者contextでは集計を読めず、DOM、URL、browser storage、logへ新しい秘密露出がないことを実ブラウザーで確認する。API、SSR HTML、repository projectionは自動testで確認済み。
- [x] 独立レビューを反映し、README、Story、対応表、検証記録、Surprise & Discoveryを更新する。

## concern

- 回答件数は本人確認済みの人数ではない。同名や同一人物の複数回答を技術的には区別せず、画面でも「人」ではなく「回答」と表現する。
- 主催者capabilityをlocalStorageから読むため、同じoriginでscriptが実行できるXSSは主催者権限にも到達できる。
- localStorageは利用者設定で拒否または削除される。復旧キーを再入力できても、紛失した秘密をサーバーから復元することはできない。
- 回答とひとことは別transactionで保存される。完全なsnapshotでも、回答が見えて、その直後のひとことがまだ見えない状態は正常である。
- ひとこと専用の保存時刻がないため、preview順はひとこと送信順ではなく、元の回答を保存した順で決定する。
- 最大3件のpreviewは通常画面を守る一方、全コメントへのアクセスを提供しない。後続の詳細表示で必要性を判断する。
- 匿名回答APIにrate limitはまだない。大量回答に対してqueryとpayloadは制限するが、公開運用前には書き込み側の保護を別途判断する。
- 専用routeのURLは秘密ではない。主催者capabilityがなければ内容を返さないことが認可境界である。
