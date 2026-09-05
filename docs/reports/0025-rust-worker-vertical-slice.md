# Rust Worker vertical slice

## 結果

`workers-rs 0.8.5` の Rust Worker を `worker-build 0.8.5` で生成し、Miniflare と一時 D1 で `/health`、イベント作成、入力検証を確認した。

```text
PASS rust-worker health + D1 event create + validation
```

`strip = "debuginfo"` を採用した。`strip = true` では wasm-bindgen の externref table が除去され、catch wrapper 生成に失敗するためである。これは workers-rs の既知の問題（https://github.com/cloudflare/workers-rs/issues/1014）に該当する。

## 範囲と未実装

この検証は公開デプロイではない。`wrangler.toml` の D1 ID はプレースホルダーのままで、認証、主催者 capability、匿名回答、CORS/CSRF、エラー形式は次の段階に切り出す。既存 DB や本番 secret は使用していない。
