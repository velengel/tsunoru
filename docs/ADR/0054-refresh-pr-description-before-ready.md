# ADR 0054: PR ready 前に本文を最終実装へ合わせる

Status: accepted

## context

#9 は実装後も本文に「空のスターターコミットのみ」と残っていた。ユーザーは PR ready の報告時に本文を最新化するよう明示した。

## decision

PR ready を報告する前にタイトルと本文を最終差分・実行した検証・未完了事項へ合わせ、GitHub から読み戻して確認する。

## rejected options

- 会話の報告だけを更新する。会話を知らないレビュアーが変更を判断できない。
- 実装予定を完了扱いで残す。検証の範囲が曖昧になる。

## consequences

レビュー修正で範囲が変わった場合も本文を更新する。GitHub の Ready flag と最終 head のレビュー完了は別に記録する。
