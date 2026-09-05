# ADR 0053: Worker 検証依存をリポジトリで固定する

Status: accepted

## context

#9 の検証は別リポジトリの Miniflare を絶対パスで import していたため、新規 checkout で再現できない。検証スクリプトの停止時には workerd を残さない必要もある。

## decision

Rust Worker の検証は worker-build 0.8.5 と lockfile に固定した Wrangler・Miniflare を使う。

## rejected options

- koji-todo の node_modules を使い続ける。他プロジェクトの更新に結果が依存する。
- ツールを毎回 latest で解決する。検証済みの組合せが変わる。

## consequences

Node の開発依存が増える。Wrangler 4.129.0 が依存する Miniflare 5.20260903.0-alpha を同じ版で固定する。alpha 名を持つ検証基盤の更新リスクは、lockfile と HTTP 回帰試験で管理する。D1 はメモリ内の fixture を使い、正常終了・例外・シグナルで作成した workerd を dispose する。公式 crate は標準の cargo bin にインストールする。
