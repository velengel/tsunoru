# ADR 0078: D1 Time Travel を staging の復旧演習に使う

## context

Cloudflare D1 は production backend の Time Travel を常時有効にし、過去の bookmark へ復元できる。staging D1 は `version: production` 相当の新 storage backend で、データ量は小さい。restore は D1 全体を上書きするため、通常運用の自動 backup と区別して扱う必要がある。

## decision

staging の復旧演習は D1 Time Travel の bookmark restore を disposable データで行い、実 staging restore は復元前 bookmark と明示承認なしに実行しない。

## rejected options

Worker の scheduled export だけを backup とする案は、失敗時の検出と復元点の保証が弱く、D1 の標準 point-in-time recovery を使わないため退ける。実 staging を無承認で restore する案は既存の合成データと利用中の状態を上書きするため採用しない。

## consequences

Time Travel は自動的に bookmark を作るため日次 backup job は不要だが、Free plan では復元可能期間が7日、Paid plan では30日に制限される。restore 後は30日 cleanup、削除済みデータの再削除、漏えい capability の交換を別手順として実行する必要がある。復元演習の RPO/RTO と失敗通知は運用記録へ残す。

## references

- [Cloudflare D1 Time Travel and backups](https://developers.cloudflare.com/d1/reference/time-travel/)
- [Cloudflare D1 limits](https://developers.cloudflare.com/d1/platform/limits/)
- [Cloudflare D1 audit logs](https://developers.cloudflare.com/d1/observability/audit-logs/)
