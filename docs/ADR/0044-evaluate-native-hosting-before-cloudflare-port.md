# ADR 0044: 初回公開は既存サーバーを保つ構成から評価する

Status: accepted (evaluation order only)

Date: 2026-09-05

## context

目的は公開URLで予定調整を使えることにある。
現行Dioxus FullstackはAxum、Tokio、SQLxのSQLiteファイルと対話的transactionを使う。
CloudflareのRust対応だけでは既存バイナリの互換性を保証しない。
費用上限と公開対象はまだ確定していない。
根拠と比較は[公開計画](../reports/0021-publication-plan.md)に置く。

## decision

初回公開の評価は単一のネイティブRustサーバーと永続SQLiteから始める。

## rejected options

- Pagesへの静的配置だけで公開を完了する。server functionsとDBが動かない。
- 最初からWorkersとD1へ全面移植する。公開目的に対して認証とtransaction再実装の負担が大きい。
- 今回ホスティング契約まで確定する。費用と公開条件をまだ比較している。

## consequences

既存の業務処理を再利用しやすい一方、単一障害点、SQLiteの書き込み競合、バックアップ運用、デプロイ時の停止を受け入れる必要がある。
評価順序だけを決め、Renderの採用や有料契約は確定しない。
無料必須やCloudflare内完結が必要になった場合は、計画の分岐条件から再評価する。
