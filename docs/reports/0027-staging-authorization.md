# staging の入口認証と回答所有権

2026-09-06。PR #10、base `492506e`。

## 結果と検証

#9 のイベント共通鍵を廃止し、staging への入口鍵、イベントの主催者鍵、回答ごとの鍵を別に検証する。回答 ID は D1 が発行する公開識別子であり、名前や ID を知っていても既存回答を上書きできない。UI はまだ接続していない。

修正前の Worker で、鍵なしの `GET /api/events/missing` が404を返し、401を期待する HTTP テストの失敗（exit 1）を確認した。修正後の実 Worker/Wasm と D1 で以下が通った。

| 検証 | 結果 |
| --- | --- |
| 未認証401、不正 Origin403、設定欠落・不正503 | PASS。DB binding を渡さず、DB より前の拒否を確認 |
| JSON/媒体型/streamed 64KiB 上限 | PASS。400/415/413、拒否時に無書込み |
| 候補の不足・余剰・重複・不正な三値 | PASS。全候補を正確に一度ずつ要求 |
| 同名の別人、同内容再送、変更再送 | PASS。別鍵は独立回答、再送200、変更409 |
| 同じ回答を6並行で送信 | PASS。201が1件、200が5件、保存は1回答 |
| 異なる内容を同じ鍵で並行送信 | PASS。201/409、勝った回答の全候補だけを保存 |
| 2件目の候補・回答 INSERT に故障注入 | PASS。D1 batch の全件 rollback、内部 SQL を応答に出さない |
| 主催者の回答一覧 | PASS。誤った鍵403、正しい鍵200、hash と秘密値を含まない |
| SIGINT/SIGTERM | PASS。初期化中・D1 操作中の所有プロセスと一時データ消滅を確認 |
| Worker cargo check/clippy/fmt、worker-build | PASS |
| `cargo test --all-targets` | PASS、118件 |
| `cargo test --all-targets --features server` | PASS、226件 |
| `cargo clippy --all-targets --all-features -- -D warnings`、`cargo fmt --check` | PASS |
| `dx build --web` | PASS、client/server build 完了 |
| Wrangler staging dry-run | PASS。620.97 KiB / gzip 216.51 KiB、実デプロイなし |

再現手順: `cloud/rust-worker/README.md`。Worker 0.8.5、Wrangler 4.129.0、Miniflare 5.20260903.0-alpha を固定した。Miniflare 5 は旧 options を直接受け取らず、公式 `convertV4MiniflareOptions` と明示した JS/Wasm module を使った。

## 範囲と理由

全候補保存は [D1 batch](https://developers.cloudflare.com/d1/worker-api/d1-database/#batch) の一括 commit/rollback を使う。ただし batch だけでは認可にならないため、書込み条件に capability hash・event・payload hash・候補集合を含めた。秘密の保存は [Workers secrets](https://developers.cloudflare.com/workers/configuration/secrets/) に従い、コード内の比較は固定長 SHA-256 と `subtle`、本文上限は `futures-util` の stream 読取で処理する。

今回の API は明示送信の Bearer を使い、Cookie 認証と CORS を導入していない。共有鍵を持つ限定検証者のための API であり、個人別の入口認証や一般公開用の rate limit を保証しない。新規 D1 用 schema のみで、旧データの自動変換はしない。

実配置と Dioxus UI は [#11](https://github.com/velengel/tsunoru/issues/11)、一般公開の制限・失効・migration の要否は [#12](https://github.com/velengel/tsunoru/issues/12) に分けた。実際の Cloudflare 配置、スマホの実機操作は未実施。

## worktree 整理

main を #9 のマージ `492506e` に更新。取り込み済み・clean・非稼働の `cloudflare-event-api` worktree と `feat/cloudflare-event-api`、`spike/cloudflare-runtime` のローカル branch を非 force の Git 操作で削除した。ignored は再生成可能な build/target/cache のみ。未追跡文書のある `calendar-pr-ready` は保持した。新規 worktree はマージ済み作業と次 PR を分けるために作成した。

## PR の確認

実装は [7f2e6e0](https://github.com/velengel/tsunoru/commit/7f2e6e02d8ca323deda7a9d90766717d151620b8) として push 済み。初回 Codex review はこの head で完了し、summary・reviews・inline threads を照合して指摘0件だった。ローカルレビューは [R040–R044](../review-judge-logs.md) に記録した。必要な検査はローカルで実行しており、確認時の GitHub CI checks は0件。

[PR #10](https://github.com/velengel/tsunoru/pull/10) の本文を実装・検証・未実施の配置範囲に合わせて更新し、GitHub から読み戻した。最終 head のレビュー完了は [Codex summary](https://github.com/velengel/tsunoru/pull/10#issuecomment-5553168882) の対象 commit と照合する。ready flag 単独を承認の根拠にしない。
