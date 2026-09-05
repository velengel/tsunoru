# ADR 0048: DioxusのAPI契約をWorkerで受ける

## context

Dioxus serverのWasm化はmioで失敗したがbrowserはビルドできる。
JS入口とRust Wasmの既存domain/Argon2計算はlocal workerdで成功している。
ユーザーは画面、認証、Cloudflare上の一連動作まで進めることを依頼した。

## decision

Dioxus browserの既存JSON API契約をJavaScript WorkerとD1で受け、入力検証とArgon2はRust Wasmで再利用する。

## rejected options

- Dioxus serverをforkする。初回接続にframework保守を持ち込む。
- UIを全面的に書き換える。既存UIを活かせる。
- 実験用固定認証を公開する。本物の権限境界を検証できない。

## consequences

SQLx実装とD1実装が当面並存し、HTTP契約と認可の回帰試験が必要になる。
Rust Wasmは同期呼出しのJSON境界に限定し、JS側で生成したrandom salt、session、capabilityを使う。
初回のCloudflare環境は検証用とし、一般公開の運用条件の完了とは分ける。
