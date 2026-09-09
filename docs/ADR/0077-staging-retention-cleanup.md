# ADR 0077: staging 合成イベントを30日で scheduled cleanup する

## context

staging を継続利用すると合成イベント、回答、rate limit 行が蓄積する。主催者の即時削除だけでは削除忘れを防げず、無期限保持は後続の運用・復旧判断を曖昧にする。

## decision

staging の作成済みイベントを30日保持し、毎日一回の scheduled cleanup でイベント関連行を原子的に削除する。

## rejected options

無期限保持は D1 の肥大化と不要な個人名の残存を招くため退ける。アクセス時の遅延削除は誰も開かないイベントを残し、主催者 capability を持つ利用者だけの手動削除は削除忘れを防げないため採用しない。

## consequences

期限後のstaging合成イベントは復元できず、主催者や回答者は再作成が必要になる。scheduled 実行と D1 batch の運用を追加し、既存行の migration では作成時刻を現在時刻として扱う。production データの保持期間・削除窓口・監査は別途定義し、staging の自動削除をそのまま本番条件にはしない。
