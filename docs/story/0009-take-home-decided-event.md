# Story 0009: 決定した予定を見て持ち帰る

Status: in progress

Date: 2026-09-02

Product sequence: Story 7

## context

Product Story 6で、主催者は候補日時を一つ明示的に確定できるようになった。
現時点の決定は主催者用private summaryだけにあり、共有URLを持つ回答者は結果を確認できない。

日程が決まった勢いのまま、参加者が決定日時を確認し、一件の予定として自分のcalendarへ持ち帰り、同じ共有URLを必要な相手へ渡せるようにする。
calendar account全体との同期や、新しいイベント管理機能には広げない。

## definition of done

- 決定済みeventの共有URLをログインなしで開くと、event名、決定日時、eventのタイムゾーンが明確に見える。
- 主催者用画面でも同じ決定日時から、持ち帰りと共有の次の行動へ進める。
- 未決定eventは決定済みと表示せず、既存の匿名回答経路を短いまま保つ。
- 決定した一eventだけをiCalendar fileとして取得できる。
- iCalendarはevent名、安定したUID、生成時刻、保存した開始ローカル日時とIANAタイムゾーンから解決したUTC開始時刻を持ち、仕様にない終了時刻や所要時間を補わない。
- event名などをiCalendarへ埋め込む際、改行、区切り文字、長いUTF-8 textを壊さず、calendar injectionを防ぐ。
- 未決定event、存在しないevent、壊れたdecisionから、誤ったcalendar fileを返さない。
- 共有操作はaccount登録を要求せず、対応browserではsystem share、非対応または失敗時には同じ共有URLをcopyできる。
- calendar追加と共有は決定結果の後に置き、回答前の必須操作や主催者の日程決定へ混ぜない。
- 320pxとdesktopで決定日時、calendar download、共有、失敗とfallbackをkeyboardから確認できる。
- Google Calendar等のaccount全体の同期、calendar読取、通知、出欠更新、ログイン、履歴を追加しない。
- 利用者に見える受け入れtestと、public projection、iCalendar生成、HTTP responseのtestを実装より先に追加し、期待した理由でREDになる。
- test、Clippy、format、Fullstack build、秘密情報検査が成功する。
- 8081の候補版を検証している間も、検証済みserverを別portで利用できる。

## to do

- [x] First InstructionのProduct Story 7と、Story 6・8の境界を確認する。
- [x] iCalendar、public projection、download response、共有fallbackを一次情報から調査する。
- [x] calendar text、公開範囲、cache、共有UI、未決定・失敗境界をADRへ記録する。
- [x] 決定済み・未決定のpublic表示、calendar download、共有・copy fallbackの失敗する受け入れtestを書く。
- [x] iCalendar escaping、line folding、timezone、終了時刻省略、未決定拒否の失敗するtestを書く。
- [x] public decision projectionとiCalendar responseを実装する。
- [x] 回答者用と主催者用の決定結果へcalendar・共有controlを実装する。
- [ ] 320pxとdesktopの実ブラウザーで、決定表示、download、共有fallback、keyboardを確認する。
- [x] 匿名回答の最短経路と主催者capability非露出が変わらないことを確認する。
- [x] 独立レビューを反映し、README、Story、対応表、検証記録、Surprise & Discoveryを更新する。

## concern

- 候補には開始日時しかなく、calendar fileへ根拠のない終了時刻や所要時間を追加してはいけない。
- `TZID` を使うlocal date-time、UTCへの変換、`VTIMEZONE` のどれを採るかで、DSTとcalendar clientの解釈が変わる。固定offsetの `VTIMEZONE` は独立レビューで撤回し、一件のUTC instantへ解決する。
- iCalendarのTEXT escaping、CRLF、75 octetsのline foldingを誤ると、event名が壊れたり別propertyを注入できたりする。
- public-by-linkのprojectionへdecisionを加えると、回答者全員が結果を読めるようになる。主催者capability、確定記録時刻、回答詳細まで公開してはいけない。
- Web Share APIとClipboard APIはsecure context、browser support、user activation、permissionの条件が異なる。share失敗を完了と誤表示しないfallbackが必要になる。
- `.ics` downloadをserver functionで返す場合、Dioxus生成client用のJSON responseと、browser navigation用のcalendar responseの境界を混同しない必要がある。
- organizer画面と回答者画面へ別々の決定表示を作ると、文言、timezone、calendar file、共有URLがずれる可能性がある。
- Story 8のloginと履歴を先取りせず、匿名の一eventだけを持ち帰る縦の体験に留める。
