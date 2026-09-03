# Story 0013: 月間カレンダーから候補日を選べる

Status: in progress

Date: 2026-09-02

## context

イベント作成では、候補ごとに日付inputと時刻inputを埋め、「候補に追加」を押す必要がある。
候補日が複数あるほど、曜日と日付の関係を別のcalendarで確かめながら同じ操作を繰り返すことになる。

調整さんの現行Web版は、候補を直接入力する経路に加え、calendarの日付をclickして候補へ加える経路を持つ。
TSUNORUでも月全体を見たまま候補日を選べれば、曜日を見比べる視線と候補追加の操作を同じ場所に置ける。

一方、時刻までcalendar cellへ詰め込むと、狭い画面で日付が押しにくくなる。
時刻は一つの基準時刻として文字入力できるようにし、最初は夕方の集まりに使いやすい `19:00` を入れる。

## definition of done

- 通常のイベント作成と継続イベント作成で、当月の月間calendarを常に見ながら候補日を選べる。
- 前月・次月buttonで表示月を移動でき、年月の変化を支援技術へ通知する。
- calendarの日付buttonを一度押すと、その日と現在の基準時刻を候補へ追加する。同じ日時をもう一度押すと解除する。
- 選択済みの日付buttonは、色だけでなくtextと `aria-pressed` で状態を伝える。
- 基準時刻は `HH:MM` の文字inputで、初期値を `19:00` とする。変更は以後のcalendar選択へ使い、追加済み候補を黙って書き換えない。
- 日付の直接入力と追加buttonをfallbackとして残し、calendarで表示しにくい日も候補にできる。
- 不正な時刻、重複、21件目は候補へ加えず、既存のdomain上限と同じerrorを表示する。
- 追加済み候補は日時順に表示し、一件ずつ削除できる。
- 320pxでpage全体を横overflowさせず、日付buttonは高さ44px、横幅24px以上とし、長い文言を折り返す。
- calendarを不完全なARIA gridとして宣言せず、native buttonのTab・Enter・Space操作を保つ。
- calendar算術、追加・解除、初期値、HTML semantics、responsive contractのtestを実装より先に追加する。
- test、Clippy、format、Fullstack build、秘密情報検査が成功する。

## to do

- [x] 現行の通常作成、継続作成、候補validationを確認する。
- [x] 調整さん公式helpとWAIのdate picker例を確認する。
- [x] calendar選択、基準時刻、fallback、keyboard境界をADRへ記録する。
- [x] calendar算術と候補toggleの失敗するtestを書く。
- [x] 通常作成と継続作成の失敗するUI testを書く。
- [x] 月間calendarと共有候補pickerを実装する。
- [ ] 320pxとdesktopでoverflow、focus、選択状態を確認する。
- [x] README、Story、検証記録、Surprise & Discoveryを更新する。

## concern

- 日付buttonを31個すべてTab stopにすると操作数は増える。初期実装ではnative buttonの予測可能性を優先し、実測後にroving tabindexを判断する。
- `role="grid"` を付けるだけでは矢印key、Home、End、Page Up/Downの契約を満たせない。
- 基準時刻を変えたときに既存候補まで更新すると、利用者が個別に追加した時刻を失う。
- browserの現在日取得前は当月を確定できない。SSRへserver timezoneの月を埋めず、hydrationまでは準備中表示と直接入力を残す。
- calendar clickと直接追加が別のvalidationを持つと、不正な日時や上限の扱いがずれる。
