# ADR 0080: staging の正規 origin を設定として共有し drift を検知する

## context

Custom Domain 移行後の正規 staging origin は `https://staging.tsunoru.velengel.com` だが、Worker 設定、fixture、README、検証手順に URL の重複がある。重複は変更時の origin drift と、旧 `workers.dev` URL の再利用を招く。

## decision

正規 staging origin と移行中 fallback を共有設定に定義し、Worker 設定・fixture・active docs を検査する drift verifier を worker check に組み込む。

## rejected options

旧 URL を削除して検索で管理する案は、移行中の fallback と過去の検証証跡を区別できない。TOML の値を実行時に外部ファイルから生成する案は Wrangler の設定可読性と dry-run の再現性を下げるため採用しない。

## consequences

正規 origin の変更には共有設定、Wrangler、active docs の更新が必要になり、check が未更新箇所を失敗として示す。旧 `workers.dev` は既存利用者の移行期間中に残し、fallback の削除は全リンク移行と HTTP 確認後の別変更とする。履歴レポートは当時の endpoint を保持する。
