# ADR 0067: 一般公開前の運用境界を分割して判断する

## context

Issue #12は、限定stagingの次に必要な公開前判断を一つにまとめている。現在のWorkerは共有staging鍵、主催者capability、回答capability、専用D1を持つが、一般公開向けの濫用対策・利用者別失効・旧SQLite移行は提供しない。

## decision

一般公開は、rate limit/abuse対策、利用者別失効、D1 backup/restoreを個別ADRに対応する後続Issueで実装・検証してから判断し、旧SQLiteの自動移行とorigin越えcapability引き継ぎは行わない。

## rejected options

- 現在の共有staging鍵のまま一般公開する：漏えい時に利用者単位で止められず、濫用を区別できない。
- 旧SQLiteを自動変換してD1へ取り込む：所有権・capability hash・欠損データの対応が未定義で、不可逆な誤移行を招く。
- capabilityを別originへCookieやURLで引き継ぐ：共有URL経由で主催者・回答者の秘密が拡散する。
- account/Cookieを先に全面導入する：匿名回答の導線とCSRF境界を同時に変え、今回の限定stagingの目的を越える。

## consequences

Issue #12自体は判断完了として閉じられるが、一般公開は未達である。後続Issueでは、rate limitとabuse計測、利用者別失効、D1のバックアップ・復元演習をそれぞれ実装する必要がある。限定stagingの合成データは継続検証に使えるが、productionデータや旧SQLiteは触らない。旧SQLiteの利用者はD1へ自動移行されず、必要ならイベントを再作成する。origin移行時は旧originのcapabilityを新originで利用できず、再回答または再発行が必要になる。回答編集は既存capability境界で実装済み、account/CSRF全面移行は匿名導線を保つ別判断とする。

## 分割ADR

- [ADR 0069](0069-rate-limit-and-abuse-controls.md): rate limitと濫用計測
- [ADR 0070](0070-capability-revocation.md): 利用者別capability失効
- [ADR 0071](0071-d1-backup-restore.md): D1 backup/restore
- [ADR 0072](0072-migration-and-origin-capability.md): 旧SQLite移行とorigin越えcapability
- [ADR 0073](0073-response-edit-and-account-csrf-boundary.md): 回答編集とaccount/CSRF境界
- [ADR 0074](0074-retention-and-deletion-policy.md): 保持・削除運用

## follow-up issues

- rate limitとabuse対策
- 利用者別の回答・主催者capability失効
- D1 backup/restoreと保持ポリシー
