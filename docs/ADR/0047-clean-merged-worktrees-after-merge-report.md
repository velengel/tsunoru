# ADR 0047: マージ報告時に取り込み済みworktreeを整理する

## context

ユーザーはマージ報告の都度worktreeとbranchを整理し、判断が必要なものだけ確認するよう指定した。
追跡ファイルがcleanでもignoredの検証証拠が残り得る。

## decision

マージ報告時はmain同期後に安全が確認できるマージ済みworktreeとlocal branchを整理する。

## rejected options

- 全対象を毎回確認する。既に明示された整理依頼を繰り返す必要がない。
- 未追跡文書や未マージbranchまで自動削除する。未完了作業を失う。

## consequences

ancestor、clean、使用中process、ignored証拠を確認する手間が生じる。
必要証拠は退避して照合する。不明な対象は保持しユーザーへ確認する。
main、現在の作業場所、remote branchは自動削除しない。
