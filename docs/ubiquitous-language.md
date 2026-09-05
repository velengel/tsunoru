# Ubiquitous Language

この用語集は、Story、ADR、実装、テストで同じ語を同じ意味に保つための正本である。
未決のプロダクト概念は、要件が決まる前に追加しない。

## TSUNORU

- 同義語：つのる、`tsunoru`。
- 意味：友人や仲間との集まりについて、主催者が参加者から都合をつのり、開催日を決める日程調整アプリケーション。
- 使われ方：画面上のプロダクト名には `TSUNORU`、crate名とリポジトリ名には `tsunoru` を使う。
- 参考リンク：[First Instruction](../first-instruction.md)

## TSUNORUマーク

- 同義語：favicon、アプリアイコン。
- 意味：三つの明るい点が一つの橙色の点へ集まる形で、参加者の都合をつのって一つの開催日を決める流れを表す識別記号。
- 使われ方：ブラウザーのタブでTSUNORUを見分けるfaviconに使う。ホーム画面用iconや大判のロゴは、別途決定するまで含まない。
- 避けられないシステム上の同義語：HTMLのlink relationでは `icon`、asset filenameでは `favicon.png` を使う。
- 参考リンク：[ADR 0022](ADR/0022-use-a-gathering-mark-as-the-favicon.md)

## アプリケーションシェル

- 同義語：ベース画面。
- 意味：Dioxus がブラウザーに描画する最小のルートUI。プロダクト機能、ルーティング、データアクセスはまだ含まない。
- 使われ方：開発サーバーとテストがRustフロントエンドの入口を正しく扱えることを示す対象として使う。
- 参考リンク：[ADR 0003](ADR/0003-use-dioxus-for-the-rust-web-foundation.md)

## Story

- 意味：一つの利用者価値または開発成果について、背景、完了条件、作業、懸念を結び付ける要求記録。
- 使われ方：実装を始める前に `docs/story/` へ作り、検証済みの状態を checkbox に反映する。
- 参考リンク：[Story 0001](story/0001-establish-react-foundation.md)

## ADR

- 同義語：Architecture Decision Record。
- 意味：採用した意思決定を、理由、却下案、受け入れる不利益とともに残す記録。
- 使われ方：意思決定を実装する前に `docs/ADR/` へ追加し、後から変える場合は後続 ADR で更新する。
- 参考リンク：[ADR 0001](ADR/0001-use-vite-typescript-and-vitest-for-react-foundation.md)

## ローカル実体化ゲート

- 同義語：materialization preflight。macOSのsystem上では未実体化fileを`dataless`と表す。
- 意味：Gitがrepository全体を読む前に、Git objectと作業fileがMacへdownload済みかmetadataだけで確認する停止点。
- 使われ方：`dataless`が一件でもあればGit操作を始めず、File Provider管理下ではFinderの「ダウンロードを保持」を要求する。file内容を開いてdownloadを暗黙に開始する検査には使わない。
- 参考リンク：[ADR 0024](ADR/0024-require-local-git-materialization-before-worktree-creation.md)、[Apple: Work with folders and files in iCloud Drive](https://support.apple.com/guide/mac-help/work-with-folders-and-files-in-icloud-drive-mchl1a02d711/mac)

## worktree作成lock

- 同義語：repository単位作成lock。Git内部のworktree lockは避けられないsystem上の別名であり、同じ意味ではない。
- 意味：複数sessionが同じrepositoryで`git worktree add`を重ねないよう、作成中の一件だけに所有権を与える一時directory。
- 使われ方：PID、開始時刻、branch、作成先を記録し、後続作成はexit code 75で止める。Gitの`locked initializing`は不完全な管理情報をpruneから守る状態として区別する。
- 参考リンク：[ADR 0024](ADR/0024-require-local-git-materialization-before-worktree-creation.md)、[Git: git-worktree](https://git-scm.com/docs/git-worktree)

## 開発サーバー

- 意味：Dioxus CLIがクライアントとサーバーをビルドし、ローカルでブラウザーへ配信するプロセス。
- 使われ方：Fullstack構成では `dx serve --web` で起動し、外部へのデプロイとは区別する。
- 参考リンク：[Dioxus: Getting Started](https://dioxuslabs.com/learn/0.7/getting_started/)

## 受け入れテスト

- 意味：内部の関数構造ではなく、利用者が受け取るHTMLがStoryの完了条件を満たすか確かめるテスト。
- 使われ方：実装より先に書き、見出しや説明など利用者に見える手掛かりでUIを検証する。
- 参考リンク：[Story 0002](story/0002-rebuild-foundation-with-dioxus.md)

## 静的検査

- 同義語：lint。
- 意味：アプリケーションを実行せずソースコードを解析し、誤りや保守上の問題を検出する検査。
- 使われ方：`cargo clippy --all-targets --all-features -- -D warnings` を実行し、テストとWebビルドとは別の検証結果として扱う。
- 参考リンク：[Clippy Documentation](https://doc.rust-lang.org/clippy/)

## Rustフロントエンド

- 意味：画面のコンポーネントと振る舞いをRustで記述し、WebAssemblyとしてブラウザーで動かすフロントエンド。
- 使われ方：TSUNORUではDioxusのブラウザー側コンポーネントを指す。同じcrateにあるFullstackサーバー側Rustとは、Cargo featureと実行環境で区別する。
- 参考リンク：[ADR 0003](ADR/0003-use-dioxus-for-the-rust-web-foundation.md)

## イベント

- 意味：主催者が実現したい一つの集まりと、その日程調整をまとめた単位。
- 使われ方：イベント名、主催者のひとこと、候補日時、回答、決定日時を結び付ける。
- 参考リンク：[First Instruction: イベント作成](../first-instruction.md#5-イベント作成)

## 主催者

- 意味：実現したい集まりを持ち、イベントを作成して、回答を材料に開催日を決める人。
- 使われ方：最終的な日程の決定権を持つ役割を表し、システムが代わりに決定する対象にはしない。
- 参考リンク：[First Instruction: 主催者が決める](../first-instruction.md#8-主催者が決める)

## 回答者

- 意味：共有URLからイベントを開き、候補日時への都合を返す人。
- 使われ方：ログインを必須とせず、名前と回答だけで最短経路を完了できる役割を表す。
- 参考リンク：[First Instruction: 回答体験](../first-instruction.md#6-回答体験)

## 候補日時

- 意味：主催者がイベントの開催候補として提示する開始日時。初期MVPでは終了日時を含めない。
- 使われ方：主催者がカレンダーUIから選び、回答者が各候補へ○、△、×を返す。
- 参考リンク：[First Instruction: 候補日時](../first-instruction.md#候補日時)

## 回答

- 同義語：出欠回答。
- 意味：回答者が候補日時ごとに示す○、△、×の都合と、回答者名をまとめたもの。
- 使われ方：○は行ける、△は条件次第またはたぶん行ける、×は難しいことを表す。
- 参考リンク：[First Instruction: ○、△、×](../first-instruction.md#--)

## 都合

- 同義語：`availability`。
- 意味：一つの候補日時に対して回答者が示す、行ける、条件次第・たぶん行ける、難しい、のいずれか。
- 使われ方：画面では意味とともに○、△、×で選び、実装とDBでは `available`、`maybe`、`unavailable` として扱う。
- 参考リンク：[ADR 0008](ADR/0008-store-anonymous-response-as-an-aggregate.md)

## 回答capability

- 同義語：回答secret、`response_capability`。
- 意味：同じ匿名回答の再試行を識別し、その回答に続く任意操作を認可する、ブラウザーが生成したランダムな秘密。
- 使われ方：同じ内容の再試行中と回答直後だけ生の値をcomponent stateで保持し、DBにはSHA-256 hashを保存する。回答者名や内部の連番は代わりに使わない。
- 参考リンク：[ADR 0008](ADR/0008-store-anonymous-response-as-an-aggregate.md)

## ひとこと

- 同義語：コメント。
- 意味：主催者または回答者が、日程だけでは伝わらない意思や返事の感触を任意で添える短い発話。
- 使われ方：未入力でも作成や回答を完了でき、情報量の少ない挨拶も価値のある返事として扱う。回答者のひとことは、回答完了後に一回答へ0件または1件だけ追記する。
- 参考リンク：[First Instruction: ひとことを大切にする](../first-instruction.md#7-ひとことを大切にする)、[ADR 0009](ADR/0009-attach-one-optional-message-to-anonymous-response.md)

## 共有URL

- 意味：イベントを受け取った人が、ログインせず回答画面を開くためのURL。
- 使われ方：主催者がイベント作成後に参加者へ渡し、回答者はそのURLから最短回答経路へ入る。主催者capabilityは含めない。
- 参考リンク：[First Instruction: 初期MVP](../first-instruction.md#15-初期mvp)

## 公開イベントID

- 同義語：`public_id`。
- 意味：共有URLの中でイベントを識別する、高いランダム性を持つ文字列。
- 使われ方：`/events/{public_id}` の一部として使う。URLを知る人の閲覧を許す識別子であり、主催者認可用の秘密には使わない。
- 参考リンク：[ADR 0005](ADR/0005-separate-public-event-access-and-organizer-capability.md)

## public-by-link

- 意味：共有URLを知る人が、accountや追加のpasswordなしでイベントを閲覧できる公開範囲。
- 使われ方：匿名回答の短さを保つ公開方針を示す。完全な非公開または検索公開とは区別する。
- 参考リンク：[ADR 0005](ADR/0005-separate-public-event-access-and-organizer-capability.md)

## 主催者capability

- 同義語：主催者secret、`organizer_capability`。
- 意味：ログインなしの利用者が、そのイベントの主催者であることをサーバーへ示すランダムな秘密。
- 使われ方：公開イベントIDと分けてブラウザーへ保存し、日程決定など主催者専用操作でだけ使う。DBにはhashを保存する。
- 参考リンク：[ADR 0005](ADR/0005-separate-public-event-access-and-organizer-capability.md)

## 復旧キー

- 同義語：主催者capability。
- 意味：ブラウザーへ主催者capabilityを自動保存できなかったとき、主催者権限を失わないため一度だけ手動保存できる形で示す秘密。
- 使われ方：回答用の共有URLとは分け、作成成功画面を閉じる前に安全な場所へ保存する。
- 参考リンク：[ADR 0005](ADR/0005-separate-public-event-access-and-organizer-capability.md)

## イベントのタイムゾーン

- 同義語：`time_zone`、IANAタイムゾーン。
- 意味：候補日時のローカル日付と時刻を、どの地域の時間として解釈するか示すIANAの識別名。
- 使われ方：作成したブラウザーから自動取得してevent単位で保存し、候補表示とiCalendar出力に使う。
- 参考リンク：[ADR 0006](ADR/0006-store-candidate-start-time-with-event-time-zone.md)

## 回答サマリー

- 意味：回答aggregateの件数、候補日時ごとの○、△、×の件数、判断補助ラベル、回答者のひとことpreviewをまとめた主催者向け表示。
- 使われ方：主催者capabilityを確認した専用画面で、主催者の判断を補助する。日程を自動決定する推薦、回答者別の集計表には使わない。
- 参考リンク：[First Instruction: 集計画面](../first-instruction.md#9-集計画面)、[ADR 0010](ADR/0010-authorize-and-project-organizer-response-summary.md)

## 判断補助ラベル

- 意味：候補日時の○、△、×の件数から確実に言える事実を、一段だけ解釈して短く示す文。
- 使われ方：全回答が○、×が0件、×が1件、○が単独最多のいずれかを示す。score、順位、推薦、日程決定には使わない。
- 参考リンク：[First Instruction: 推薦](../first-instruction.md#推薦)、[ADR 0010](ADR/0010-authorize-and-project-organizer-response-summary.md)

## 集計表

- 同義語：従来型集計表。
- 意味：回答者と候補日時を縦横に並べ、個々の回答を確認できる表。
- 使われ方：回答サマリーだけでは判断できない場合にも情報を失わないため、主催者が必要なときだけ開く詳細表示として使う。日程を選ぶcontrolやコメント本文は含めない。
- 参考リンク：[First Instruction: 集計画面](../first-instruction.md#9-集計画面)、[ADR 0011](ADR/0011-lazily-load-an-accessible-organizer-response-matrix.md)

## 日程決定

- 同義語：確定。
- 意味：主催者が回答を材料に、候補日時から開催日時を一つ選ぶ行為。
- 使われ方：推薦や多数決では自動実行せず、主催者の明示操作によってイベントの決定日時を一件記録する。同じ候補への再試行は同じ結果を返すが、別候補で黙って上書きしない。
- 参考リンク：[First Instruction: 主催者が決める](../first-instruction.md#8-主催者が決める)、[ADR 0012](ADR/0012-make-the-first-explicit-event-decision-immutable.md)

## 決定済みイベント

- 意味：主催者が候補日時から一つを選び、変更不能な日程決定が保存されたイベント。
- 使われ方：共有URLでは匿名回答formを終え、決定日時、calendarへの持ち帰り、共有の次の行動を表示する。
- 参考リンク：[Story 0009](story/0009-take-home-decided-event.md)、[ADR 0014](ADR/0014-publish-one-decided-event-as-a-self-contained-calendar.md)

## 持ち帰り

- 意味：決定済みイベントの一件を、自分のcalendarへ追加できるfileまたは必要な相手へ渡せる共有URLとして受け取ること。
- 使われ方：calendar account全体を同期せず、決定直後の一件だけを次の行動へ移す操作を指す。
- 参考リンク：[First Instruction: 決定後の体験](../first-instruction.md#10-決定後の体験)、[ADR 0014](ADR/0014-publish-one-decided-event-as-a-self-contained-calendar.md)

## iCalendarファイル

- 同義語：`.ics` file。
- 意味：決定済みイベント一件の開始日時、event名、識別子をiCalendar形式で表すdownload file。
- 使われ方：保存したイベントのタイムゾーンで開始日時を一つのinstantへ解決し、fileにはUTCの開始日時として記録する。OAuthやcalendar account同期を要求せず、端末のcalendar applicationへ一件を渡す。
- 参考リンク：[RFC 5545](https://www.rfc-editor.org/rfc/rfc5545.html)、[ADR 0014](ADR/0014-publish-one-decided-event-as-a-self-contained-calendar.md)

## 履歴

- 意味：主催または参加したイベントについて、日程調整の過程で自然に生まれた情報を後から確認できる記録。
- 使われ方：ログインした利用者へ追加価値として提供し、記録のための新しい入力作業は要求しない。
- 参考リンク：[First Instruction: 履歴](../first-instruction.md#11-履歴)

## イベントの痕跡

- 同義語：`event trace`。実装上の型名でだけ使う。
- 意味：event名、候補日時、回答者名、候補ごとの回答、任意のひとこと、日程決定として、日程調整の過程で既に自然に生まれた情報。
- 使われ方：新しい記録入力を要求せず、login中に主催または回答したeventのprivateな履歴詳細でだけ振り返る。写真、後日の感想、reaction、timelineは含めない。
- 参考リンク：[First Instruction: 履歴](../first-instruction.md#11-履歴)、[Story 0011](story/0011-revisit-the-natural-trace-of-an-event.md)

## account

- 同義語：アカウント。
- 意味：任意のloginによって、利用者が主催または回答したイベントの履歴をまとめる単位。
- 使われ方：イベント作成と回答の前提にはせず、login中の操作から履歴への戻り道だけを加える。主催者capabilityや回答capabilityの代わりにはしない。
- 参考リンク：[Story 0010](story/0010-use-account-history-without-changing-anonymous-flow.md)、[ADR 0015](ADR/0015-keep-account-history-optional-and-server-session-bound.md)

## login ID

- 同義語：`login_id`。
- 意味：passwordと組み合わせて一つのaccountへloginするため、利用者が決める公開可能な識別名。
- 使われ方：ASCII小文字へ正規化して一意に保存する。回答者名やイベント内の表示名とは結び付けない。
- 参考リンク：[ADR 0015](ADR/0015-keep-account-history-optional-and-server-session-bound.md)

## login

- 同義語：ログイン。
- 意味：login IDとpasswordを検証し、browserへ新しいaccount sessionを結び付ける操作。
- 使われ方：履歴を利用したい人だけが行う。匿名作成や匿名回答の入口にはしない。
- 参考リンク：[First Instruction: ログイン](../first-instruction.md#11-履歴)、[ADR 0015](ADR/0015-keep-account-history-optional-and-server-session-bound.md)

## account session

- 同義語：session、ログインsession。
- 意味：login済みのbrowser requestを一つのaccountへ結び付け、serverが期限と失効を判定できる一時的な状態。
- 使われ方：browserにはHttpOnly cookieで生token、SQLiteにはそのhashだけを持つ。履歴関連付けに使い、主催者専用操作の認可には使わない。
- 参考リンク：[ADR 0015](ADR/0015-keep-account-history-optional-and-server-session-bound.md)

## 主催履歴

- 意味：account sessionが有効な間に、そのaccountが作成したイベントの一覧。
- 使われ方：イベント名、決定日時、回答件数を短く示し、既存のpublic eventへ戻す。主催者capabilityは履歴項目へ含めない。
- 参考リンク：[First Instruction: 履歴](../first-instruction.md#11-履歴)、[ADR 0015](ADR/0015-keep-account-history-optional-and-server-session-bound.md)

## 参加履歴

- 意味：account sessionが有効な間に、そのaccountが新しく回答したイベントの一覧。
- 使われ方：イベント名と決定日時を短く示し、既存のpublic eventへ戻す。同じイベントへの複数回答は一覧では一件にまとめる。
- 参考リンク：[First Instruction: 履歴](../first-instruction.md#11-履歴)、[ADR 0015](ADR/0015-keep-account-history-optional-and-server-session-bound.md)

## 継続イベント

- 同義語：シリーズ。
- 意味：同じ活動が回数や時期を変えて繰り返されるイベントのまとまり。
- 使われ方：login中の主催者が過去eventから「同じ活動の次回をつのる」と明示した場合だけ作る。event名が変わっても明示的な関係から主催履歴をまとめ、初回または通常のevent作成時の必須分類にはしない。
- 参考リンク：[First Instruction: シリーズ、継続イベント](../first-instruction.md#12-シリーズ--継続イベント)、[Story 0012](story/0012-continue-an-explicit-event-series.md)、[ADR 0017](ADR/0017-create-series-only-from-explicit-account-continuation.md)

## 次回名の候補

- 意味：明示的に継続イベントを続けるとき、現在の末尾event名が厳密な `名前 #N` なら、連番を一つ進めてevent名inputへ入れる編集可能な初期値。
- 使われ方：通常作成または履歴を開いただけでは表示せず、主催者が同じ活動を続けると選んだ後にだけ示す。利用者は採用、編集、削除でき、候補自体を保存済みの命名規則にはしない。
- 参考リンク：[First Instruction: シリーズ、継続イベント](../first-instruction.md#12-シリーズ--継続イベント)、[ADR 0017](ADR/0017-create-series-only-from-explicit-account-continuation.md)

## 候補日calendar

- 同義語：候補日カレンダー。画面文言では漢字表記を使う。
- 意味：一か月の曜日と日付を見ながら、候補日時へ加える日を直接選べる入力領域。
- 使われ方：通常作成と継続作成で、日付buttonと基準時刻を組み合わせて候補を追加または解除する。日付の直接入力を置き換えず、同じ候補listへつなぐ。
- 参考リンク：[Story 0013](story/0013-pick-candidate-dates-from-an-inline-calendar.md)、[ADR 0019](ADR/0019-use-an-inline-month-calendar-with-an-editable-base-time.md)

## 基準時刻

- 意味：候補日calendarの日付を押したとき、その日と組み合わせる編集可能な開始時刻。
- 使われ方：`HH:MM` の文字inputへ初期値 `19:00` を入れる。変更後に選ぶ候補へだけ使い、追加済み候補を一括変更しない。
- 参考リンク：[Story 0013](story/0013-pick-candidate-dates-from-an-inline-calendar.md)、[ADR 0019](ADR/0019-use-an-inline-month-calendar-with-an-editable-base-time.md)

## 回答後一覧

- 同義語：みんなの回答。
- 意味：一件の回答が保存された直後に、そのeventの回答者と全候補の○、△、×を確認できる表。
- 使われ方：回答成功POSTのpayloadから同じ画面に表示する。共有URLを開いただけでは返さず、主催者用集計表とは認可入口を分ける。
- 参考リンク：[Story 0014](story/0014-see-everyones-answers-after-submitting.md)、[ADR 0020](ADR/0020-return-the-response-matrix-only-after-a-successful-answer.md)

## Finder task UI

- 同義語：Finderの作業用window、情報panel、詳細panel、sheet。macOSのsystem上ではFinder windowまたはaccessibility windowとして現れる。
- 意味：repository調査のため、その作業中に新規作成または一時的に再利用したFinderのUI。
- 使われ方：操作前のwindow ID、target、bounds、panelとの差分で識別する。作業後は新規UIを閉じ、再利用した既存windowを復元して、無関係な利用者windowを残す。
- 参考リンク：[Story 0018](story/0018-prevent-dataless-git-worktree-stalls.md)、[ADR 0024](ADR/0024-require-local-git-materialization-before-worktree-creation.md)

## File Provider domain

- 同義語：iCloud Drive管理領域。macOS上では`com.apple.file-provider-domain-id` metadataで識別する。
- 意味：iCloud Driveなど、macOS File Providerがlocal placeholder、download、uploadを管理するfilesystem領域。
- 使われ方：File Provider内のrepositoryではKeep Downloadedとpreflightを要求する。正本とlinked worktreeの作成先には使わない。
- 参考リンク：[Apple: iCloud Driveのfileとfolderを操作する](https://support.apple.com/guide/mac-help/work-with-folders-and-files-in-icloud-drive-mchl1a02d711/mac)、[ADR 0024](ADR/0024-require-local-git-materialization-before-worktree-creation.md)

## 配信asset整合性

- 同義語：`asset provenance`。検証codeと技術文書でだけ使う。
- 意味：browserが同じbuildから生成されたHTML、Wasm、stylesheetを取得し、sourceと対応する状態。
- 使われ方：live HTMLが参照するhash付きstylesheetの内容と固有selectorを確認してから、computed layoutとscreenshotをvisual evidenceとして採用する。
- 参考リンク：[Story 0015](story/0015-repair-and-prove-the-served-calendar-layout.md)、[ADR 0021](ADR/0021-isolate-served-assets-before-visual-verification.md)、[Dioxus 0.7: Assets](https://dioxuslabs.com/learn/0.7/essentials/ui/assets/)

## PR ready

- Meaning: the user-facing completion state after planning, implementation, required checks, self-review fixes and final-head Codex-review dispositions have converged and the PR is mergeable.
- Usage: deliver the PR link only with that completed evidence; a pending external review remains pending work.
- System synonym: GitHub calls its draft transition "Ready for review", but that flag alone is not this completion state.
- Reference: [ADR 0030](ADR/0030-require-codex-review-convergence-for-pr-ready.md), [Story 0022](story/0022-converge-codex-pr-review.md).
