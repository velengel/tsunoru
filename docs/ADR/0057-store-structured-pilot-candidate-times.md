# ADR 0057: 限定版でも日時候補を構造化して保存する

## context

#10 の候補は自由な label であり、native UI の local_date / local_time / time_zone と対応しない。表示文字列を解析して日時へ戻すと、利用者の locale や曖昧な入力で意味が変わる。日程調整には同じ候補を主催者と回答者が同じ意味で読む必要がある。

## decision

限定版の候補を日付・時刻とイベントの IANA time zone で保存し、Worker で実在する日時を検証する。

## rejected options

- label へ日時を埋めて後から解析する。型とタイムゾーンの意味を失う。
- ブラウザーの入力検査だけに任せる。API へ直接送った未知 time zone や不正日時を拒否できない。
- 全 native DTO と SQL schema を同時移植する。今回使わない account 等に依存する。

## consequences

String の候補 ID は維持し、日付と時刻を別フィールドで返す。候補はイベントの time zone を明示して表示する。存在しない日付や時刻、未知の time zone、DST で一意に定まらない日時は拒否する。D1 は新規の専用 DB に限るため、今回の変更を既存データの migration とは扱わない。主催者のひとことは既存フォームと同じ上限で任意入力として扱う。

根拠: `src/domain.rs` の NewEventInput と PublicCandidate、[chrono TimeZone](https://docs.rs/chrono/latest/chrono/trait.TimeZone.html)、[chrono-tz](https://docs.rs/chrono-tz/latest/chrono_tz/)。
