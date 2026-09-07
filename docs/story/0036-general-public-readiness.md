# Story 0036: 一般公開前の制限・失効・移行を判断する

## context

Issue #12。限定stagingで成立した匿名イベント機能を一般公開へ広げる前に、濫用対策、回答編集と失効、認証、D1移行、capability境界を判断する。

## definition of done

- 各論点をfix / defer / no-changeと理由付きでADRへ記録する。
- 実装が必要な論点は独立Issueへ分割し、未実装の公開条件を明記する。
- 旧SQLiteの自動移行やproductionデータへの操作を行わない。

## to do

- [x] rate limitとabuse対策の公開前条件を決める
- [x] 回答編集・失効・account/CSRFの境界を決める（編集はdefer、現行CSRFは維持）
- [x] D1 migrationとbackup/restoreの扱いを決める
- [x] origin越えcapabilityと旧SQLiteの扱いを決める
- [x] 実装候補を優先度付きの後続Issueへ分割する（#26–#29）

## concern

限定stagingのローカル・実URL検証は一般公開の安全性を証明しない。公開判断と実装完了、デプロイ証拠を混同しない。
