# Cloudflare Worker bridgeの初回接続

2026-09-05。PR #8の同一worktreeで、Dioxus browser側とRust Wasmの境界をWorkerへ置く最小接続を追加した。

`cloud/worker/src/index.ts` は本番APIではなく、Rust Wasmのdomain検証とArgon2計算結果をJSONで返すdiagnostic endpointだけを持つ。
固定saltや認証、D1の読み書き、既存画面の公開は含めない。
Cloudflareの検証用D1 bindingはplaceholder IDのため、デプロイは実行しない。

次の実装では、このendpointを製品APIへ拡張せず、まずD1の合成schemaと入力境界をWorker testで置き換える。
既存のDioxus server functionをそのまま公開していないため、UIとAPIの完全接続はまだUNVERIFIEDである。

Rust Workerの最小crateを`cloud/rust-worker`へ追加した。
`workers-rs 0.8.5`の`fetch`、Router、D1 bindingでhealthとイベント作成を定義する。
schemaは合成イベント一表だけで、本番migrationや認証は含めない。

Rust coreのWasm buildはPASS（410 KiB）。既存Workers型を使ったTypeScriptの型検査は、リポジトリ外のWorkers型とTypeScript標準DOM型の重複によりFAILした。実行コード固有の型エラーと環境設定エラーを区別し、cloud/workerの専用依存を導入する次段階で再検証する。
