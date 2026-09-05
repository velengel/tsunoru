# Cloudflare staging event API

Rust Worker にイベント作成と匿名回答の最小 API を追加した。イベント作成時に主催者・回答者 capability を保存し、回答時は `x-response-capability` を Worker 側で照合する。

検証:

```text
cargo check --locked --target wasm32-unknown-unknown: PASS
cargo clippy --locked --target wasm32-unknown-unknown -- -D warnings: PASS
worker-build --release: PASS
node verify-local.mjs: PASS rust-worker health + D1 event create + validation
```

本番 resource、secret、domain は使用していない。Cookie session、Origin/CSRF、rate limit、UI接続は次の検証対象である。
