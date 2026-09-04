# Story 0020: 移行済みの旧iCloud sourceを削除する

Status: complete

Date: 2026-09-04

## context

TSUNORUの正本は`$HOME/Developer/active/tsunoru`へ移り、public GitHubのmainと未完了のカレンダーfeature branchからsourceを復元できる。
SQLiteとquizはGitHubへ含めず、新しい正本だけに置いていた。

旧iCloud repositoryと元のworktreeはrollback用として残していたが、利用者は最終照合後の完全削除を承認した。
旧Git履歴は復元対象にせず、StoryとADRおよび新しい公開履歴を今後の記録とする。

## definition of done

- 旧rootと各worktreeのclean、dirty、detached状態を削除前に再確認する。
- 各worktreeの変更が新しいmainまたはカレンダーfeature branchへ回収済みであることを、tree ID、正規化diff、file checksumで確認する。
- canonical SQLiteとquizをFile Provider外へ別途保全し、checksumとSQLite整合性を確認する。
- dirty worktreeの変更fileを復元用に別途保全する。
- 旧repositoryへ連結したworktreeを一件ずつ解除してから、旧iCloud rootを削除する。
- 新しい正本、カレンダーworktree、GitHub、local stateが削除後も正常であることを確認する。
- 削除結果とrollback境界をrepository reportと中央migration台帳へ記録する。

## to do

- [x] 旧rootと5件のworktreeを棚卸しする。
- [x] ADR簡潔化、実体化ゲート、公開snapshot、カレンダー変更の回収先を照合する。
- [x] Git lockと旧pathを使うprocessを調べる。
- [x] canonical SQLite、quiz、dirty worktreeの変更fileをFile Provider外へ保全する。
- [x] 4件のlinked worktreeを直列に解除する。
- [x] 旧iCloud rootを削除し、pathが存在しないことを確認する。
- [x] canonical Git、worktree、SQLite、quiz、GitHub状態を再検証する。
- [x] 検証記録、中央migration台帳、Surprise & Discoveryを更新する。

## concern

- 旧source削除後は117commitのlocal履歴へ戻れない。
- sourceの復元元はpublic GitHubとcanonical cloneへ変わる。
- SQLiteとquizはGitHubへ含まれないため、canonical copyと削除前backupを両方失うと復元できない。
- 削除中にsystem metadataが再生成されると、一回の`rm -rf`が一部directoryを残す場合がある。
- 実行中のCodex sessionは旧rootをcwdとして保持していたため、削除後のcommandはすべてcanonical pathを明示して実行する。

## Understanding Gate（削除前理解確認）

- Status: `Passed`
- Reason: 旧履歴と旧worktreeを消す不可逆操作であり、削除後のsourceとlocal stateの復元元を先に固定する必要があるため。
- Questions: 旧iCloud sourceと元worktreeを最終照合後に完全削除してよいか、GitHubへ含まれないlocal stateを保全するかを確認した。
- User explanation: iCloud側は移行後に完全削除する。旧Git履歴は捨ててよく、StoryとADRから判断を追える。復元できるもの、または影響が小さいものは削除してよい。
- Misalignment / Resolution: GitHubだけではSQLiteとquizを復元できないため、canonical copyと同一hashの削除前backupをFile Provider外へ作ってから削除した。
- Unresolved: なし。
