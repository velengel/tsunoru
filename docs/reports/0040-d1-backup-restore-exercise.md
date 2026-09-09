# Report 0040: staging D1 backup/restore 演習

## 実施結果

2026-09-09、`tsunoru-staging` の現在 bookmark `0000000a-00000000-000050e1-bafd6236c30130467ab67e87b0d4c732` を取得した。専用 route の一時 `rate_limits` 行を作成した後、その bookmark へ in-place restore し、行が `0` 件へ戻ることを確認した。

復元後、`events`、`responses`、`answers` テーブルの存在を確認し、staging Worker（`https://tsunoru-staging.kounakadora528.workers.dev/`）が HTTP 200 を返した。

## 境界

Time Travel restore は D1 全体を上書きし、in-flight query と transaction をキャンセルする。今回扱ったのは staging の合成データだけで、production の restore は実行していない。復元前 bookmark と Wrangler が示した undo bookmark は運用記録として保持する。

## 検証コマンド

- `npx wrangler d1 time-travel info tsunoru-staging --env staging`
- `npx wrangler d1 execute tsunoru-staging --remote --command "..."`
- `npx wrangler d1 time-travel restore tsunoru-staging --bookmark=...`
- `curl -fsS -i https://tsunoru-staging.kounakadora528.workers.dev/`
