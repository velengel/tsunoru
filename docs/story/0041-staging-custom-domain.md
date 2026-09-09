# Story 0041: staging をサービス配下の URL へ移行する

## context

staging の利用 URL から Cloudflare アカウント由来の識別子を外し、`velengel.com` 配下でサービス名と環境を表現する。

## definition of done

- `staging.tsunoru.velengel.com` が `tsunoru-staging` Worker の Custom Domain になる。
- Worker の APP_ORIGIN と staging の実 URL が新 hostname に一致する。
- 新 URL で HTTP、staging session、イベント作成・共有・回答の疎通を確認する。
- 旧 workers.dev URL の扱いを記録する。

## to do

- [ ] Wrangler 設定へ Custom Domain と APP_ORIGIN を追加する
- [ ] staging Worker を再配置する
- [ ] 新 URL を実ブラウザと HTTP で確認する
- [ ] Google OAuth 設定の更新要否を確認する

## concern

Custom Domain は `velengel.com` zone が現在の Cloudflare アカウントにある場合に限り設定できる。既存の共有リンクや cookie の host scope は hostname 変更後に再確認する。
