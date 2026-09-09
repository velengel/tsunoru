# ADR 0079: staging はサービス配下の Custom Domain を使う

## context

現在の staging URL は `kounakadora528.workers.dev` を含み、サービス名と環境名が URL から分かりにくい。`velengel.com` は Cloudflare に委任され、`staging.tsunoru.velengel.com` は未使用である。

## decision

staging の正規 URL を `https://staging.tsunoru.velengel.com` とし、`tsunoru-staging` Worker の Custom Domain として運用する。

## rejected options

既存の `workers.dev` URL を正規リンクとして維持する案は、Cloudflare アカウント由来のサブドメインを利用者へ露出し、将来の本番 URL と環境の階層を表現しにくいため退ける。`tsunoru-staging.velengel.com` は有効だが、サービス配下に環境を置く命名規則との一貫性が弱いため採用しない。

## consequences

既存の workers.dev URL は移行期間の確認用に残りうる。APP_ORIGIN、Google OAuth の許可済み origin、ブラウザ検証 URL は新しい hostname へ更新する。Custom Domain 設定自体の追加料金はなく、`velengel.com` の zone が同じ Cloudflare アカウントで管理されていることが前提になる。
