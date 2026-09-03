# ADR 0019: 編集可能な基準時刻と月間calendarで候補を選ぶ

## context

通常作成と継続作成は、日付input、時刻input、「候補に追加」の三操作を候補ごとに繰り返す。
曜日を含む月全体が見えないため、複数候補を選ぶほどcalendar applicationとの往復が増える。

調整さん公式helpでは、日付・時刻の直接入力に加え、calendarの日付clickでも候補を入力できる。
2024年の公式案内ではdefault時刻を変更でき、時刻なしも選べる。TSUNORUのdomainは開始時刻を必須にしているため、時刻なしへは広げず、editableな基準時刻を採用する。

WAIのDate Picker Dialog例はcalendar gridの完全なkeyboard操作を示す一方、例示コードには実環境の支援技術で検証するよう注意がある。
TSUNORUはdialogを開かず、作成form内へ複数選択のcalendarを常時表示する。矢印keyを実装しない段階で `role="grid"` を名乗ると、支援技術へ未実装の操作契約を示してしまう。

参考:

- [調整さんhelp: イベントを作成する](https://help.chouseisan.com/ja/articles/9969027-%E3%82%A4%E3%83%99%E3%83%B3%E3%83%88%E3%82%92%E4%BD%9C%E6%88%90%E3%81%99%E3%82%8B)
- [調整さん: デフォルト時刻設定](https://chouseisan.com/l/post-132401/)
- [WAI APG: Date Picker Dialog Example](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/examples/datepicker-dialog/)
- [WAI APG: Grid Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/)
- [WCAG 2.2: Target Size (Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)

## decision

- 通常作成と継続作成は同じ候補picker componentを使う。時刻の初期値、calendar算術、validation、候補listのmarkupを重複させない。
- 基準時刻は `type="text"`、`inputmode="numeric"`、最大5文字の `HH:MM` inputとし、初期値を `19:00` にする。native time pickerだけに操作を委ねない。
- calendarの日付buttonは、現在の基準時刻と組み合わせた候補をtoggleする。同じ日・同じ時刻があれば削除し、なければ追加する。
- 基準時刻の変更は既存候補を書き換えない。異なる時刻で同じ日を追加することは許し、calendarの選択表示は現在の基準時刻との完全一致で決める。
- calendar追加と直接追加は、同じ `CandidateInput` の正規化、日付・時刻shape、重複、最大20件を使う。追加後は日付・時刻の昇順へ並べる。
- browserのlocal dateから初期表示月を決める。SSRではlocal monthを推測せず準備中表示にし、直接入力fallbackは使用可能なままにする。
- 表示月は1年1月から9999年12月までとし、前月・次月buttonを境界で無効にする。Gregorian calendarの閏年と曜日を整数演算で求め、timezoneやserver機能へ依存させない。
- calendarは年月heading、曜日heading、空cell、native day buttonをCSS gridへ並べる。年月は `aria-live="polite"`、月移動buttonは完全なaccessible nameを持つ。
- day buttonは `aria-pressed` と「候補に追加／候補から削除」を含むaccessible nameを持つ。選択状態を色だけにしない。
- `role="grid"`、roving tabindex、矢印key contractは実装しない。各dayをnative buttonとしてTab、Shift+Tab、Enter、Spaceで操作できる状態を正とする。
- day buttonは高さ44pxを保ち、横幅はWCAG 2.2の24px以上を下限としてcalendar全体を7等分する。320pxでもpage横scrollを発生させない。
- 直接入力fallbackには既存のdate inputと追加buttonを残す。calendar外の日付と、同日の別時刻も入力できる。

## rejected options

### native `input type="date"` だけを残す

browserごとのpickerを利用できるが、候補日を複数見比べ、選択済みの分布を月全体で確認できない。今回の操作負担を解消しないため却下する。

### calendar cellごとに時刻inputを置く

31個の重複inputでTab stopと視覚密度が増え、320pxで日付buttonのtargetを保てないため却下する。

### 基準時刻を固定の19:00にする

夕方以外の集まりで候補ごとの修正が必要になる。初期値にだけ使い、利用者が文字で変更できるようにする。

### 基準時刻を変えたら既存候補も一括変更する

追加済み候補が利用者の確認なしに変わり、同日の別時刻も表現できなくなるため却下する。

### WAIのdate picker gridを一部だけ実装する

矢印key、Home、End、Page Up/Down、roving tabindexを含まない `role="grid"` は、見た目だけを借りて操作契約を破る。まずnative buttonとして完成させる。

### JavaScriptのdate libraryを追加する

必要なのは月移動、月の日数、曜日だけである。依存とclient bundleを増やさず、小さなpure Rust関数を境界値testで固定する。

## consequences

- 複数候補を月の文脈で選びやすくなる。一方、31個前後のday buttonがTab順へ入るため、keyboard利用者の移動量は増える。
- 19:00は開始点にすぎず、地域や利用目的に合わない場合がある。常に見えるtext inputから変更できることで受け入れる。
- hydration前はcalendarを表示できない。直接入力fallbackにより作成機能そのものは失わない。
- Gregorian算術をapplicationが持つ。閏年、月境界、年境界をpure testで固定し、日時instantやtimezone変換には流用しない。
- `aria-pressed` は現在の基準時刻との完全一致を示す。基準時刻を変えると以前の候補はlistに残るがcalendar上のpressed表示は変わる。
- 将来roving tabindexと矢印keyを追加する場合、実ブラウザーとscreen readerの検証を伴う別ADRで `role="grid"` への変更を判断する。
