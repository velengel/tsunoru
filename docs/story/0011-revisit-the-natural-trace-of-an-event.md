# Story 0011: 履歴詳細でイベントの痕跡を振り返れる

Status: in progress

Date: 2026-09-02

Product sequence: Story 9

## context

Product Story 8で、login中に主催または回答したeventが二つの短い履歴へ自然に積み上がるようになった。
一覧はevent名、決定日時、回答件数だけに留めたため、過去の調整で誰がどう返し、どんなひとことが添えられたかまでは振り返れない。

新しい活動記録を書かせるのではなく、候補日時、回答、ひとこと、日程決定として既に保存された情報を、必要なときだけ履歴詳細で読めるようにする。
写真、感想投稿、reactionを加える思い出SNSにはせず、日程調整の過程で生まれた痕跡だけを扱う。

## definition of done

- login中の利用者は、主催履歴と参加履歴の各項目からprivateな履歴詳細を開ける。
- 履歴一覧は短いまま保ち、回答者名、候補ごとの回答、ひとことを一覧へ展開しない。
- 履歴詳細にはevent名、任意の主催者のひとこと、候補日時、eventのtimezone、調整中または決定日時を表示する。
- login中に主催したeventでは、届いた回答の回答者名、候補ごとの当時の回答、任意のひとことを確認できる。
- login中に回答したeventでは、そのaccountへ結び付いた自分の回答者名、候補ごとの当時の回答、任意のひとことだけを確認できる。
- 一つのaccountが同じeventへ複数回答している場合は、いずれか一件を推測して隠さず、結び付いた回答を区別して確認できる。
- 主催と参加の両方に該当する場合も役割を混同せず、利用者自身の回答を識別できる。
- 他accountの参加履歴、anonymous response、主催関係のないeventから、回答者名、回答、ひとことを読めない。
- event不存在とaccountに閲覧関係がない場合を同じmissing stateへ揃え、private dataの有無をerrorで区別しない。
- account sessionは履歴詳細のreadだけを認可し、日程決定、主催者summary、回答更新のcapabilityにはしない。
- SSR HTMLへprivateな履歴詳細を埋め込まず、hydration後にloading、未login、session期限切れ、missing、失敗、表示成功を区別する。
- event作成、回答、ひとこと、日程決定に新しいfieldまたは保存操作を加えず、既存のanonymous flowを変えない。
- 写真、感想投稿、reaction、timeline、series、次回名の提案を先取りしない。
- 320pxとdesktopで長いevent名、回答者名、ひとこと、候補日時を読め、keyboardで履歴へ戻りpublic eventも開ける。
- domain、repository、server function、認可、UI、responsiveのtestを実装より先に追加し、期待した理由でREDになる。
- test、Clippy、format、Fullstack build、秘密情報検査が成功する。
- 8081の候補版を検証している間も、検証済みserverを別portで利用できる。

## to do

- [x] First Instructionの履歴、Story 9、Story 10との境界を確認する。
- [x] 現行のevent、response、ひとこと、日程決定、account関連付けを確認する。
- [x] Dioxusのtyped route・server function、requestごとの認可、SQLite snapshotを一次情報から確認する。
- [x] privateな履歴詳細、役割別の可視範囲、既存aggregate再利用、失敗stateをADRへ記録する。
- [x] domain、repository、server認可の失敗するtestを書く。
- [x] route、loading・missing・失敗・詳細表示、responsiveの失敗するtestを書く。
- [x] 既存aggregateから最小の履歴詳細projectionを作る。
- [x] account sessionで毎回認証・認可するtyped server functionを実装する。
- [x] 履歴一覧からprivate詳細へ進み、public eventにも移れるmobile-first UIを実装する。
- [x] anonymous flow、public event、organizer capabilityの境界が変わらないことを確認する。
- [ ] 320pxとdesktopの実ブラウザーで二つの役割、各state、長い内容、keyboard操作を確認する。
- [x] 独立レビューを反映し、README、Story、対応表、検証記録、Surprise & Discoveryを更新する。

## concern

- public eventへ回答者名、回答、ひとことを足すと、共有URLを知る全員へprivateな痕跡を広げてしまう。
- 参加履歴のdetailでevent全体の回答を返すと、一参加者が他人の回答者名とひとことを読める。
- accountが主催した関係をread認可に使う判断と、主催者capabilityによる操作認可を混ぜると、Story 8の権限境界が崩れる。
- 一覧と詳細で同じ大きなprojectionを返すと、履歴の初期表示とmobileの可読性を損なう。
- 複数回答を最新一件だけへ暗黙にまとめると、実際に残った痕跡と異なる表示になる。
- response、availability、comment、decisionを別々の時点で読むと、一画面の中で矛盾したsnapshotになり得る。
- private detailをSSRへ埋め込むと、共有端末のHTML、cache、view sourceへaccount dataを残しやすい。
- 大きなeventの全回答を返すdetailはresponse sizeが増える。初期実装ではevent単位の全件を受け入れ、実測なしにpaginationや検索を先取りしない。
