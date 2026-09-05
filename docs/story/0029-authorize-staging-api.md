# Story 0029: staging API のアクセスと回答の権限を分離する

## context

#9 のマージ後、Cloudflare 上で限定利用を始めるための実装へ進む。#9 ではイベント共通の capability と自由指定の回答 ID で上書きでき、全候補を回答した保証もなかった。既存 native 実装と同じく、回答ごとの capability と内容の一致で再送を判定する。

## definition of done

- [x] main を #9 マージへ更新し、別 worktree と開発 PR #10 を作成する。
- [x] staging の鍵が未設定なら閉じ、認証と Origin の拒否を DB 操作より前に行う。
- [x] 全候補への回答を一度に保存し、同内容の再送・同名の別人・競合を区別する。
- [x] 公開用イベント情報と主催者向け回答一覧を分離する。
- [x] HTTP の拒否・同時送信・rollback を修正前の失敗から確認する。
- [x] 本文、Story、判断ログを実装に合わせて更新し、PR ready の根拠を記録する。

## to do

- [x] #9 のマージと worktree の安全を確認する。
- [x] ADR、API 契約、固定した検証依存を用意する。
- [x] 回帰試験の失敗を確認してから Worker と schema を実装する。
- [x] ローカル HTTP、Rust、アプリの回帰検査、Wrangler dry-run を実行する。
- [x] ローカルレビューと受信済み Codex 指摘を判断する（自動追従は最大2往復）。

## concern

今回の完了範囲は staging API とデプロイ手順の準備。Cloudflare 資源の作成、実デプロイ、UI 接続は次作業へ残す。データは新規の隔離 D1 に限り、旧実験 DB を自動変換しない。一般公開用のレート制限・Cookie account・回答編集・権限失効は導入前に別 Issue で扱う。#9 の個人学習クイズは `.mydocs/` に置き PR へ含めない。

引き継ぎ先: [staging 配置と UI #11](https://github.com/velengel/tsunoru/issues/11)、[一般公開前の判断 #12](https://github.com/velengel/tsunoru/issues/12)。

実装 `7f2e6e0` の Codex review は完了し、summary・reviews・inline threads を確認して指摘0件だった。最終 head の外部レビュー状況は [PR #10 の summary](https://github.com/velengel/tsunoru/pull/10#issuecomment-5553168882) と突き合わせる。検証根拠は [report 0027](../reports/0027-staging-authorization.md)、判断は [R040–R044](../review-judge-logs.md) に記録した。
