# Story 0040: staging D1 の backup/restore を演習する

## context

Issue #29。30日保持と削除運用を追加した staging で、障害や誤更新から復旧できることを証明する。Cloudflare D1 の Time Travel は自動 bookmark と point-in-time restore を提供するが、restore はデータベース全体を上書きするため、合成データと明確な復元条件が必要になる。

## definition of done

- D1 backend、Time Travel の保持期間、復元主体、RPO/RTO を記録する。
- disposable D1 で marker 作成、bookmark 取得、restore、復元後の marker 確認を演習する。
- restore 後に30日保持 cleanup、削除済みデータの再削除、失効済み capability の交換条件を確認する。
- backup の鮮度、容量、失敗通知、実データ適用前のロールバック条件を記録する。
- staging の合成データだけを扱い、production の restore は実行しない。

## to do

- [x] Cloudflare D1 Time Travel の現行仕様と破壊境界を確認する
- [x] staging D1 の backend と容量・利用量を確認する
- [x] disposable 環境で restore 演習を実行する
- [x] RPO/RTO、監視、再適用手順を記録する

## concern

Time Travel restore は対象 D1 を in-place で上書きし、in-flight query をキャンセルする。今回の演習では、復元前 bookmark を保存し、専用の一時 `rate_limits` 行を restore で消去した。production の restore は実行していない。
