# Story 0026: Cloudflare実行の難所を小実験で確かめる

## context

PR #7の計画をマージし、Cloudflare優先の実現性を検証する。
本番移植を始める前にDioxus/Wasm、D1の原子性、現行Argon2設定を切り分ける。

## definition of done

- [x] main同期、マージ済みworktree整理、新規worktreeとdraft PR #8を作成する。
- [x] 現行/最小DioxusのWasm互換性を実コマンドで判定する（server FAIL、browser PASS）。
- [x] 隔離したworkerdでD1保存、失敗rollback、競合、Argon2計算を試す（JS入口＋Rust Wasm。Rust Worker bindingと本物のloginは未検証）。
- [x] PASS/FAIL/UNVERIFIED、再現コマンド、次の選択肢を[結果](../reports/0023-cloudflare-runtime-spike.md)へ記録する。
- [x] self-reviewと必要修正をまとめる。push状態はPRで確認する。

## to do

- [x] 期待するHTTP試験をworker実装前に置き、ENOENTの失敗を確認した。
- [x] 依存単位のビルドと実行証拠を収集した。
- [x] 公開移植を自動で続けず、小実験の結果で終了する。

## concern

実験は合成データとlocal D1から始める。本番/他repoのDBとsecretは使わない。
ローカルworkerd成功はCloudflare上のCPU制限や費用の証明ではない。
古いcalendar-pr-readyは未追跡文書があるためユーザー判断待ちで保持する。
レビューの自動対応は最大2往復とし、失敗を隠して成功範囲を広げない。
