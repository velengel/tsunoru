# ADR 0050: staging API は capability をサーバー側で検証する

Status: accepted

Date: 2026-09-06

## context

Rust Workerの最小イベントAPIはローカル検証用で、イベントIDだけで保護操作を認可してしまう。公開に向けては、主催者と匿名回答者の能力を分離し、D1に保存した秘密値をWorkerだけが検証する必要がある。既存アプリはlocalStorageの主催者能力とセッションを持つため、UIの値だけを信頼してはならない。

## decision

分離staging APIは主催者・回答者のcapabilityをWorker内で検証してからD1を更新する。

## rejected options

- イベントIDや公開回答IDだけで更新を許可する。推測・再利用で他人のイベントを変更できるため採らない。
- capabilityをブラウザーだけで検証する。クライアント改変で回避できるため採らない。
- 既存SQLiteをstaging Workerから直接同期する。公開経路とローカルDBの責務が混ざり、失敗時の復旧境界が不明確になるため別Issueとする。

## consequences

Worker APIとDioxus UIの両方にcapabilityの受け渡しが必要になる。秘密値の失効、Origin/CSRF、レート制限、セッションCookieは実装時に検証する。staging schemaは本番DBと分離し、匿名回答の公開projectionは必要最小限にする。
