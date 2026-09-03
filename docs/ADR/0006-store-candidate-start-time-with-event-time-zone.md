# ADR 0006: 候補日時は開始ローカル日時とイベントのタイムゾーンで保存する

Status: accepted

Date: 2026-09-01

Amended: 2026-09-02。Story 7の出力形式は[ADR 0014](0014-publish-one-decided-event-as-a-self-contained-calendar.md)で改訂した。

## context

First Instructionの候補日時は `金曜 19:00〜` のように、日付と開始時刻を中心に表現している。
終了時刻や所要時間は必須入力として定義されていない。

HTMLの日付入力と時刻入力は、利用者のローカル日時を返し、タイムゾーンを含まない。
ローカル日時だけを保存すると、別のタイムゾーンから共有URLを開いた場合や、Story 7でカレンダーへ持ち帰る場合に、どの地域の19時か分からなくなる。

## decision

- 候補日時は、ISO 8601形式のローカル日付 `YYYY-MM-DD` とローカル時刻 `HH:MM` を分けて保存する。
- イベントごとに、作成したブラウザーから取得したIANAタイムゾーン名を保存する。
- タイムゾーンは必須の手入力にせず、ブラウザーの `Intl.DateTimeFormat().resolvedOptions().timeZone` から自動取得する。
- 候補日時の順番は主催者が追加した順序を保存する。作成画面と回答画面でも同じ順序を使う。
- 同じイベント内で日付と開始時刻が完全に一致する候補は重複として受け付けない。
- 初期MVPでは終了時刻または所要時間を入力させない。
- ローカル日時を暗黙にサーバーのタイムゾーンへ変換しない。
- Story 7のiCalendarでは、保存したIANAタイムゾーンでlocal startをUTC instantへ解決する。固定offsetの `VTIMEZONE` や `TZID` は出力せず、仕様にない終了時刻を補わない。

参考資料：

- [MDN: date input](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/input/date)
- [MDN: time input](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/input/time)
- [MDN: Intl.DateTimeFormat resolvedOptions](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/DateTimeFormat/resolvedOptions)
- [RFC 5545: iCalendar](https://www.rfc-editor.org/rfc/rfc5545.html)

## rejected options

### UTCへ変換したinstantだけを保存する

時系列の比較は簡単になる。
一方、作成時の地域とローカル表現を失うと、DST規則やカレンダー出力で元の意図を復元しにくい。
初期MVPではローカル日時とIANAタイムゾーンを正本にする。

### ローカル日時だけを保存する

入力値をそのまま扱える。
しかし、別地域の利用者やカレンダーが同じ文字列を異なるinstantとして解釈するため採用しない。

### 終了時刻を必須入力にする

カレンダー上の予定枠を正確に作れる。
First Instructionにない入力を主要フローへ増やし、`19:00〜` とだけ呼びかけたい集まりを重くするため採用しない。

### 既定で一時間または二時間の長さを補う

カレンダー上では一般的な予定枠になる。
集まりの長さをシステムが根拠なく決めるため採用しない。

### 独自のカレンダーgridを最初から実装する

ブランドに合う一貫した見た目を作れる。
キーボード操作、locale、月移動、mobile pickerを同時に設計する必要がある。
Story 1では、スマートフォンが提供するcalendar pickerを使えるnative inputから始める。

## consequences

- 利用者が入力した日付と時刻を、サーバーの地域設定で変えずに表示できる。
- Story 7でOAuthなしのiCalendarを作るために必要な地域情報が残る。
- 作成フォームにタイムゾーンの手入力を増やさずに済む。
- ブラウザーがIANAタイムゾーンを返さない場合の作成エラーと案内が必要になる。
- DSTの存在しない時刻や二重に存在する時刻を厳密に検証する処理はまだ持たない。
- 終了時刻のないiCalendar eventはdurationがゼロとして解釈されうる。利用者は必要に応じてcalendar側で終了時刻を補う。
- 将来、複数タイムゾーンをまたぐ集まりやduration入力を扱う場合は、後続ADRでschemaと表示を拡張する必要がある。
