# First Instruction 実装対応表

Date: 2026-09-02

この表は、`first-instruction.md` のProduct Storyと、完了を判断する証拠を結び付ける。
基盤の完成と、利用者が使えるプロダクト機能の完成を混同しないために使う。

| Product Story | 利用者価値 | Repository Story | 状態 | 主な受け入れ証拠 |
| --- | --- | --- | --- | --- |
| 1 | 匿名でイベントを作り、共有URLを得る | [Story 0003](../story/0003-create-anonymous-event.md)、[Story 0013](../story/0013-pick-candidate-dates-from-an-inline-calendar.md) | calendar改善は実装済み・実ブラウザー未検証 | [月間calendar、自動test、HTTPはPASS、実ブラウザーはUNVERIFIED](0013-calendar-and-post-answer-matrix-verification.md) |
| 2 | 共有URLから○、△、×を回答する | [Story 0004](../story/0004-answer-availability-without-login.md)、[Story 0014](../story/0014-see-everyones-answers-after-submitting.md) | 回答後一覧まで実装済み・実ブラウザー未検証 | [回答成功時の一覧、自動test、HTTPはPASS、実ブラウザーはUNVERIFIED](0013-calendar-and-post-answer-matrix-verification.md) |
| 3 | 回答後に任意のひとことを返す | [Story 0005](../story/0005-add-an-optional-message-after-answering.md) | 実装済み・実ブラウザー未検証 | [自動testとHTTPはPASS、実ブラウザーはUNVERIFIED](0005-optional-response-message-verification.md) |
| 4 | 主催者が回答サマリーを見る | [Story 0006](../story/0006-view-organizer-response-summary.md) | 実装済み・実ブラウザー未検証 | [自動test、WAL snapshot、HTTPはPASS、実ブラウザーはUNVERIFIED](0006-organizer-response-summary-verification.md) |
| 5 | 回答者と候補日時の集計表を見る | [Story 0007](../story/0007-view-response-matrix.md) | 実装済み・実ブラウザー未検証 | [自動test、WAL snapshot、HTTPはPASS、実ブラウザーはUNVERIFIED](0007-organizer-response-matrix-verification.md) |
| 6 | 主催者が日程を決定する | [Story 0008](../story/0008-decide-event-candidate.md) | 実装済み・実ブラウザー未検証 | [自動test、競合・再open、実HTTPはPASS、実ブラウザーはUNVERIFIED](0008-organizer-event-decision-verification.md) |
| 7 | 決定した予定を持ち帰る | [Story 0009](../story/0009-take-home-decided-event.md) | 実装済み・実ブラウザー／実calendar未検証 | [自動test、DST、raw HTTPはPASS、実端末操作はUNVERIFIED](0009-decided-event-handoff-verification.md) |
| 8 | ログインして主催・参加履歴を見る | [Story 0010](../story/0010-use-account-history-without-changing-anonymous-flow.md) | 実装済み・実ブラウザー未検証 | [自動test、SQLite、実HTTPはPASS、実ブラウザーとTLS deploymentはUNVERIFIED](0010-account-history-verification.md) |
| 9 | イベントの自然な痕跡を残す | [Story 0011](../story/0011-revisit-the-natural-trace-of-an-event.md) | 実装済み・実ブラウザー未検証 | [role別認可、WAL snapshot、SSR privacy、実HTTPはPASS、実ブラウザーはUNVERIFIED](0011-event-trace-verification.md) |
| 10 | 継続イベントと命名補助を使う | [Story 0012](../story/0012-continue-an-explicit-event-series.md) | 実装済み・認証付き実HTTP／実ブラウザー未検証 | [明示的membership、並行stale、atomic rollback、SSR privacy、自動testはPASS](0012-event-series-verification.md) |

## 匿名MVPの一続きの証拠

Product Story 1から7が個別に通った後、次の操作を一つの実ブラウザーテストで確認する。

1. 主催者contextでイベントを作り、回答用共有URLを得る。
2. 独立した二つの回答者contextから○、△、×と任意のひとことを送る。
3. 主催者contextで回答サマリーと従来型集計表を確認する。
4. 主催者が候補日時を一つ選び、明示的に確定する。
5. 回答者contextで決定日時を確認し、iCalendarを得る。
6. サーバーを再起動し、イベント、回答、ひとこと、日程決定が残ることを確認する。
7. 回答者経路を320px、主催者の判断画面を320pxとdesktopで操作する。

## 基盤の状態

Dioxus Webのアプリケーションシェルは[Story 0002](../story/0002-rebuild-foundation-with-dioxus.md)で完成している。
ただし、HTTP 200と静的なシェル表示はProduct Story 1から10の証拠には数えない。
