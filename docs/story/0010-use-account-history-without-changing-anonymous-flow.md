# Story 0010: ログインすると主催・参加履歴へ戻れる

Status: in progress

Date: 2026-09-02

Product sequence: Story 8

## context

Product Story 1から7までで、イベント作成、匿名回答、主催者の判断、日程決定、calendarへの持ち帰りをログインなしで完遂できるようになった。
この縦の体験を入口から分岐させず、先にログインしていた利用者にだけ、使う過程で生まれたイベントへの戻り道を加える。

履歴のためにイベントを登録し直したり、回答後に活動記録を書かせたりしない。
ログイン中に主催または回答した事実を既存のwriteと同時に結び付け、後から一覧で辿れるようにする。

## definition of done

- 利用者はログインIDとpasswordでaccountを作成し、ログイン、ログアウトできる。
- passwordの平文とsession tokenの平文をSQLite、log、URL、HTML、browser storageへ保存しない。
- passwordは個別salt付きのmemory-hard password hashとして保存する。
- sessionはserver側で失効と期限を判定し、browserにはHttpOnly、SameSite、host限定のcookieだけを渡す。
- ログイン失敗は、accountの有無とpasswordの誤りを区別しない。
- ログイン中に作成したeventは主催履歴へ、ログイン中に新しく送った回答のeventは参加履歴へ、既存writeと同じtransactionで結び付く。
- 匿名で作成または回答した既存eventを、login IDだけを根拠に遡ってaccountへ推測・移管しない。
- 履歴は主催と参加を分け、event名、決定済みなら決定日時、主催履歴では回答数を短く表示する。
- 各履歴項目から既存のpublic eventへ戻れる。主催者用capabilityの権限をaccount sessionへ暗黙に移さない。
- 未ログイン、履歴0件、読込中、読込失敗、session期限切れ、ログアウトを区別して案内する。
- 匿名のevent作成と回答にはlogin、account作成、追加fieldを要求せず、既存の最短操作数を増やさない。
- Story 9の回答・comment詳細と、Story 10のseries・命名提案を一覧へ先取りしない。
- 320pxとdesktopでlogin、account作成、二つの履歴、空状態、失敗、logoutをkeyboardから操作できる。
- domain、password、session、repository、server function、UI、responsiveのtestを実装より先に追加し、期待した理由でREDになる。
- test、Clippy、format、Fullstack build、秘密情報検査が成功する。
- 8081の候補版を検証している間も、検証済みserverを別portで利用できる。

## to do

- [x] First Instructionのlogin、履歴、Story 8から10の境界を確認する。
- [x] password保存、session cookie、login error、SQLite関連付けを一次情報から調査する。
- [x] account、session、履歴projection、匿名境界、失効と失敗方針をADRへ記録する。
- [x] account、session、event・response関連付け、履歴queryの失敗するtestを書く。
- [x] login、account作成、履歴、logout、responsiveの失敗する受け入れtestを書く。
- [x] migration、domain、password処理、session repositoryを実装する。
- [x] typed server functionとcookie境界を実装する。
- [x] 既存のevent作成・回答へ任意のaccount関連付けを追加する。
- [x] loginと履歴のmobile-first UIを実装する。
- [x] 匿名E2Eの操作数、public data、organizer capability境界が変わらないことを確認する。
- [ ] 320pxとdesktopの実ブラウザーでaccount作成、login、履歴、logout、期限切れを確認する。
- [x] 独立レビューを反映し、README、Story、対応表、検証記録、Surprise & Discoveryを更新する。

## concern

- password authはhash algorithmだけで安全にならない。入力上限、個別salt、計算cost、account列挙、brute force、回復手段、HTTPSを一つの境界として扱う必要がある。
- local HTTPとproduction HTTPSではcookieの `Secure` 要件が異なる。開発を動かすためにproduction cookieを弱めない設定境界が必要になる。
- 生session tokenをDBへ保存するとDB漏えいだけでsessionを再現できる。server側もhashだけを保存し、logoutと期限切れを検証する必要がある。
- account関連付けがeventまたはresponseのcommitと別transactionになると、利用者には成功に見えて履歴だけ欠ける状態を作る。
- 過去の匿名eventを名前やlogin IDで自動claimすると、同名回答者や共有端末から他人の履歴を奪える。
- account sessionをorganizer capabilityの代替にすると、既存のpublic／private認可境界が変わる。Story 8では履歴からpublic eventへ戻ることに限定する。
- 履歴queryが回答やcommentを先に広く読むと、account間の情報漏えいと一覧の肥大化を招く。Story 8では最小projectionだけを返す。
- login UIをevent作成または回答formの前へ置くと、匿名利用を劣化版にしてしまう。
- password reset、email確認、外部identity provider、MFA、account削除を持たない初期実装は、公開運用前に再評価が必要になる。
