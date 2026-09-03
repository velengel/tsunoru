# ADR 0014: 決定済みイベントを一件の自己完結したcalendarとして渡す

Status: accepted

Date: 2026-09-02

## context

Product Story 7では、参加者が共有URLから決定日時を確認し、その一件だけをcalendarへ持ち帰る。
主催者も、決定直後の画面から同じ持ち帰りと共有へ進む。

公開画面は現在、候補日時と匿名回答formだけを表示する。
日程決定は主催者用projectionに閉じており、確定記録時刻、回答、主催者capabilityを回答者へ公開していない。

iCalendarでlocal date-timeへ `TZID` parameterを付ける場合、RFC 5545は同じobject内に対応する `VTIMEZONE` を要求する。
IANA名だけを `DTSTART;TZID=Asia/Tokyo` のように書く実装は短いが、自己完結したiCalendarにはならない。
当初は選択日時のoffsetだけを持つ固定offsetの `VTIMEZONE` を採用したが、独立レビューで、IANA名が表す実際のDST transitionと異なるtimezone定義になることが分かった。
存在しないlocal timeや二重になるlocal timeをcalendar clientが異なるinstantへ解釈する可能性がある。

一方、TSUNORUは終了時刻を保存しておらず、calendar accountとの同期や反復予定もStory 7の範囲に含めない。
一件のdownloadに、根拠のないdurationや将来の全タイムゾーン履歴を足す理由はない。

参考資料:

- [RFC 5545: content lines](https://www.rfc-editor.org/rfc/rfc5545.html#section-3.1)
- [RFC 5545: TZID parameter](https://www.rfc-editor.org/rfc/rfc5545.html#section-3.2.19)
- [RFC 5545: DATE-TIME](https://www.rfc-editor.org/rfc/rfc5545.html#section-3.3.5)
- [RFC 5545: TEXT](https://www.rfc-editor.org/rfc/rfc5545.html#section-3.3.11)
- [RFC 5545: VEVENT](https://www.rfc-editor.org/rfc/rfc5545.html#section-3.6.1)
- [RFC 5545: VTIMEZONE](https://www.rfc-editor.org/rfc/rfc5545.html#section-3.6.5)
- [RFC 5545: UID](https://www.rfc-editor.org/rfc/rfc5545.html#section-3.8.4.7)
- [RFC 5545: DTSTAMP](https://www.rfc-editor.org/rfc/rfc5545.html#section-3.8.7.2)
- [RFC 5545: text/calendar](https://www.rfc-editor.org/rfc/rfc5545.html#section-8.1)
- [RFC 6266: Content-Disposition](https://www.rfc-editor.org/rfc/rfc6266.html#section-4.2)
- [RFC 9111: no-store](https://www.rfc-editor.org/rfc/rfc9111.html#section-5.2.2.5)
- [Dioxus 0.7: Axum Router](https://dioxuslabs.com/learn/0.7/essentials/fullstack/axum/)
- [W3C: Web Share API](https://www.w3.org/TR/web-share/)
- [W3C: Clipboard API](https://www.w3.org/TR/clipboard-apis/#dom-clipboard-writetext)
- [WHATWG HTML: downloading resources](https://html.spec.whatwg.org/multipage/links.html#downloading-resources)

## decision

- 公開event projectionへ、候補ID、local date、local timeだけを持つ任意の公開decisionを加える。
- 公開projectionは一つのread transactionからevent、候補、decisionを読み、decisionが参照する候補を同じsnapshotで確かめる。
- 確定記録時刻、主催者capabilityとhash、回答者名、回答、ひとことは公開projectionへ加えない。
- 公開画面は未決定なら既存の匿名回答formを保ち、決定済みなら回答formを決定結果、calendar download、共有操作へ置き換える。
- 決定後に新しい回答aggregateは受け付けない。決定より前に保存済みの同一payloadを再試行した場合は、冪等な成功を返す。
- 回答直後の任意のひとことは、日程決定と競合しても保存できる。先に成立した回答に付随する操作だからである。
- iCalendarは `VCALENDAR` 一件と非反復の `VEVENT` 一件だけを持つ。`PRODID`、`VERSION:2.0`、`UID`、生成時UTCの `DTSTAMP`、保存したlocal startとIANA名から解決したUTC `DTSTART`、event名の `SUMMARY`、任意の主催者のひとことを含める。
- `UID` は生成済みのUUIDv4公開event IDから `urn:uuid:{public_id}` として作り、同じeventでは変えない。
- `DTSTAMP` はdownloadを生成したserverの現在UTC時刻を注入して作る。非公開の `event_decisions.decided_at` は使わない。
- `DTSTART` は末尾 `Z` のUTC DATE-TIMEとし、`TZID` parameterと `VTIMEZONE` は出力しない。保存したIANA名とlocal startは、serverで一つのinstantへ解決するために使う。
- DSTでlocal timeが二重になる場合はRFC 5545どおり最初の発生を使い、存在しない場合はgap直前のoffsetを使う。
- 終了時刻を保存していないため、`DTEND` と `DURATION` は出さない。`METHOD`、`ORGANIZER`、`ATTENDEE`、`VALARM` も出さない。
- iCalendarのTEXTではbackslash、comma、semicolon、改行をescapeする。CRLF、CR、LFは論理改行へ正規化し、その他の許可されない制御文字を含むdataは出力しない。
- 全content lineをUTF-8の文字境界で75 octets以下へfoldし、continuationの先頭spaceも75 octetsへ数える。改行にはCRLFを使い、file末尾もCRLFで閉じる。
- `.ics` はDioxus server functionのJSON responseにせず、`dioxus::serve`で追加する通常のAxum GET route `/api/events/{public_id}/calendar.ics` から返す。
- 成功responseは `text/calendar; charset=utf-8`、`Content-Disposition: attachment`、検証済みASCII公開IDから作るfilename、`Cache-Control: no-store`、`X-Content-Type-Options: nosniff` を持つ。
- 不正または存在しない公開IDは404、未決定eventは409、保存dataまたは生成結果の不整合は詳細を伏せた500とする。失敗responseにも `no-store` と `nosniff` を付け、calendar用headerや途中までのbodyを返さない。
- 持ち帰りUIは通常の同一origin anchorを使い、JavaScriptやhydrationがなくても `.ics` を取得できるようにする。
- 共有対象は主催者画面のURLではなく、常に `/events/{public_id}` とする。主催者capabilityをcomponent、共有payload、URLへ渡さない。
- 共有buttonはuser activationの中でWeb Shareを開始する。成功時はOSごとの差を隠して「共有操作を開始しました」と伝える。
- Web Share非対応時は同じ操作からClipboardへのcopyを試す。共有のcancelまたは失敗時は自動copyせず、buttonを明示的なcopy操作へ変える。
- Clipboardが使えない場合は、label付きreadonly inputへ同じ共有URLを表示し、手動copyできる状態を残す。
- `chrono` は `chrono-tz` が既に使う版を直接server dependencyとして宣言し、local date-time解決、offset、UTC生成時刻にだけ使う。

## rejected options

### IANA名だけをTZID parameterへ書く

主要なcalendar clientがIANA名を独自に解釈する可能性はある。
しかし、RFC 5545が要求する `VTIMEZONE` を欠き、受取側の暗黙のtimezone databaseへ結果を委ねるため採用しない。

### 選択日時のoffsetだけを持つVTIMEZONEを出力する

一件のlocal startを短い定義で表現できる。
しかし、IANA名を持つtimezoneとして実際には存在しない固定zoneを定義することになる。
DST gapとoverlapをcalendar clientがTSUNORUと同じinstantへ復元できないため、当初の決定を独立レビュー後に撤回した。

### IANAタイムゾーンの全履歴を生成するcrateを追加する

反復予定や広い期間の日時を一つの `VTIMEZONE` で解釈できる。
しかし、Story 7が出力する時刻は一件だけであり、生成codeとtzdataの大きなdependencyを初期MVPへ加える釣り合いが取れない。

### 終了時刻を一時間後として補う

calendar上で見える予定枠は作れる。
しかし、利用者が入力していない所要時間をTSUNORUが決めるため採用しない。

### Dioxus server functionまたはdata URLでdownloadする

既存のtyped RPCまたはclientだけで実装できる。
しかし、raw calendar bodyとHTTP headerの境界がJSON codecやBlob lifecycleへ依存し、JavaScriptなしのnative linkを失うため採用しない。

### event名をdownload filenameへ使う

保存後に内容を識別しやすい。
しかし、Unicode filename encodingとheader injection対策が必要になる。安全な公開IDを使う固定ASCII filenameで足りるため採用しない。

### share失敗時に自動でURLをcopyする

一回の操作でfallbackまで進める。
しかし、利用者がshare sheetを意図的にcancelした場合にもclipboardを書き換えるため採用しない。

### 決定済みeventでも回答を保存し続ける

決定直前に開いた画面から回答を送れる。
しかし、その回答は変更不能な決定へ反映されず、回答者へ誤った期待を持たせる。保存済みretryだけを成功させ、新規回答は競合として扱う。

## consequences

- 回答者と主催者は、同じ決定日時からcalendar追加と共有へ進める。
- `.ics` は一eventの開始をUTCの一意なinstantとして持ち、受取側がTSUNORUと同じtzdbを持つことを前提にしない。
- calendarは利用者側の表示timezoneで開始時刻を表示する。TSUNORUで入力したIANA名をcalendar objectへ保持する設計ではない。
- 将来、反復予定や「開催地のlocal timeを固定する」要件を導入する場合は、実際のtransitionを含む `VTIMEZONE` を生成する別設計が必要になる。
- 同じeventを再downloadするとUIDは同じだが、`DTSTAMP` は新しくなる。calendar clientが同じUIDをrevisionとして扱う可能性がある。
- 終了時刻のないDATE-TIME eventは開始時刻に終了するものとして扱われ、実質的なdurationが0になる。利用者は必要に応じてcalendar側で終了時刻を補う。
- raw Axum routeを加えるため、server起動は `dioxus::launch` から `dioxus::serve` と `dioxus::server::router` の組合せへ変わる。既存のSSR、assets、server functionsは同じrouterに残る。
- ADR 0013で見送ったcustom routerを、private server functionのdecode error補正には使わない。非HTMLのdownload resourceという別の必要性に限定して導入する。
- `no-store` はHTTP cacheへ保存しない指示であり、利用者が保存した `.ics` fileを後から消す仕組みではない。
- Web ShareとClipboardの実行結果はbrowser、OS、permissionに左右される。失敗時も決定日時、calendar link、手動copy用URLを失わない。
- 主催者用画面から共有しても、相手へ渡るのはpublic-by-linkの共有URLだけである。
- 新規回答と日程決定はDBのwrite transactionで直列化される。どちらが先にcommitしたかによって、回答が保存されるか決定済み競合になる。
