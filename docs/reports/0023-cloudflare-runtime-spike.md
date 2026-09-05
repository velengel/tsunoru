# Cloudflare小実験の結果

2026-09-05。基点main `5230bf6`。開発PR: https://github.com/velengel/tsunoru/pull/8

## 判断

Cloudflareを外す根拠はまだないが、現行Dioxus Fullstack 0.7.10をそのままWorkersへ載せる経路はビルドで失敗した。
DBと認証を除いた最小crateでも同じ失敗を再現した。
一方、JavaScriptの入口からRust Wasmを呼ぶ構成では既存のdomain検証とArgon2計算がworkerdで動き、D1の一体処理も検証できた。
この差を保ったまま次の構成判断へ進む。

## 結果と証拠

| 対象 | 結果 | 根拠と限界 |
| --- | --- | --- |
| 現行serverのWasm check | FAIL | `cargo check --locked --target wasm32-unknown-unknown --no-default-features --features server`。mio 1.2.2がunsupported wasm targetを報告 |
| 現行browserのWasm check | PASS | `cargo check --offline --locked --target wasm32-unknown-unknown`。ブラウザー側の型/依存確認であり、公開assets生成や実機操作の検証ではない |
| 最小Dioxus 0.7.10 server function | FAIL | `experiments/dioxus-worker-probe`でもmioが失敗。SQLx/Argon2なしで再現。Cargo.lockを保存 |
| 依存経路 | 確認済み | `cargo tree --offline --target wasm32-unknown-unknown --manifest-path experiments/dioxus-worker-probe/Cargo.toml -i mio --edges normal`でDioxus→Axum/Tokio→mioを確認 |
| 既存domainのRust Wasm | PASS | `src/domain.rs`をpath参照、server featureを有効化。正常入力、空名、未知timezoneをworkerdで判定 |
| D1保存と原子性 | PASS（local） | 作成/回答、重複拒否、外部キー違反、途中失敗rollback、2要求のexpected-tail競合で成功1/失敗1、失効session拒否 |
| 0行UPDATEの反例 | 再現 | batch中のUPDATEが0行でも先行INSERTはcommitする。原子性には条件不成立をSQLエラーにする設計等が必要 |
| Argon2id計算 | PASS（local） | 現行パラメータで再計算一致/別入力不一致。3計算とHTTP合計77.5msの一回観測。単発の内部wall値33ms |
| Wasm linear memory | 観測済み | 1,114,112→21,102,592 bytes。総isolateメモリや同時利用時の最大値ではない |
| Rust静的検査 | PASS | 実験crateのclippy `-D warnings`、fmt。cargo testは0件で検証根拠に数えない |
| Dioxus SSR/RPC/Cookie login | UNVERIFIED | serverビルドが止まるため未到達。合成sessionのDB検査を本物の認証成功と読み替えない |
| workers-rs binding | UNVERIFIED | worker-build取得が自動承認で拒否。既存ツールによるraw Wasm経路を別実験として確認 |
| Cloudflare実環境、CPU課金、負荷 | UNVERIFIED | デプロイ/remote D1操作をしていない。local計測から無料枠適合を断定しない |

主要エラー:

```text
This wasm target is unsupported by mio. If using Tokio, disable the net feature.
error: could not compile `mio` (lib) due to 48 previous errors
```

再現手順は[実験README](../../experiments/cloudflare/README.md)、HTTP期待値は[verify.mjs](../../experiments/cloudflare/verify.mjs)を正とする。
ログはセッション中 `/private/tmp/tsunoru-spike-evidence/` に保存した。
Miniflare `5.20260730.0-alpha`、workerd package `1.20260730.1`を既存koji-todoのnode_modulesから読み取り利用した。
新しいcompatibility dateはこのruntimeが拒否したため、対応している2026-08-06を使った。
最初の起動で生成されたCF metadata cacheは合成D1とは別であり、以後は `cf:false` とした。

## 次に行うことと行わないこと

次の実装候補はDioxusのブラウザーUIを保ち、Worker APIとD1へ接続する構成である。
今回のraw WasmはRustの業務処理を残せることの証拠に留まる。
APIをRust workers-rsで実装するか、既存アプリと同じTypeScriptにするかは未確定であり、認証境界と保守量を比べて決める。
まずイベント作成と回答の一導線だけで、UIから実APIへつなぐ。全機能の移植はまだ始めない。

Dioxus serverの依存をforkしてnet featureを除去する作業は別Issue候補とする。
さらに別の非互換が出る可能性があり、初回公開のためにframework保守まで背負う理由はまだない。
SSRが必須ならvoice-workbench型のContainersを再比較するが、永続DB設計は別に必要になる。
Renderへの切替、OAuth必須化、パスワード強度低下、実験APIの公開は行わない。

この結果は製品移植の完了ではなく、計画の不確実性を減らす小実験の完了である。
本体src、Cargo.toml、Cargo.lock、migrationは変更していないため、本体の全テストとdx buildは再実行していない。

## worktree整理

- mainをPR #7のmerge `5230bf6` にfast-forwardした。
- `cloudflare-publication-plan` / `docs/cloudflare-publication-plan` (`f4b6a64`) と `calendar-layout-verification` / `fix/calendar-layout-verification` (`d41bf59`) はmainのancestor、clean、使用中processなしを確認して削除した。
- 後者のignored画像証拠はcanonical `var/archived-worktree-evidence/calendar-layout-verification`へコピーし、`diff -qr`一致を確認した。
- `calendar-pr-ready` / `fix/calendar-pr-ready` (`12ea737`) は未マージの開始commitと未追跡文書3件があるため保持し、ユーザーへ判断を依頼した。
- remote branchは削除していない。整理ルールはADR 0047とAGENTS.mdへ記録した。

## Surprise & Discovery

公式資料のWasm配置案内と、固定バージョンの依存が実際にビルドできることは別だった。
また、domain.rsはserver featureの有無でtimezone検証が変わるため、Wasm化時にbrowser用検証へ弱めない注意が必要だった。
SQLの0行UPDATEはエラーではないことをD1で再現でき、transaction移植時の具体的な失敗例になった。

参考: [workers-rs](https://github.com/cloudflare/workers-rs/blob/main/README.md)、[D1 batch](https://developers.cloudflare.com/d1/worker-api/d1-database/)、[Workers limits](https://developers.cloudflare.com/workers/platform/limits/)。
