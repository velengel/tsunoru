# ADR 0064: Codexレビュー監視をPR単位のバックグラウンドプロセスで運用する

## context

非同期レビューは会話セッションの待機だけでは見落としやすく、レビュー完了後の指摘確認が遅れる。既存の読み取り専用監視スクリプトを、PID・ログ付きでバックグラウンド起動できる必要がある。

## decision

PR番号ごとにPIDとログを`.codex-log/codex-review-watch/`へ置くstart/stopラッパーで監視を常駐させる。

## rejected options

- 会話中だけポーリングする: セッション終了後に監視できない。
- GitHub Actionsで常時監視する: 個人開発のローカル判断ログと実行状態に対して過剰である。
- PIDを検証せず停止する: unrelated processを誤停止する危険がある。

## consequences

GitHub CLI認証が有効なローカル環境で動作し、PRごとのログを後から確認できる。stopはコマンドラインが対象Watcherと一致する場合だけ終了し、ログとPIDファイルはignored領域に残る。
