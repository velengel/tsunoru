# ADR 0061: 主催者ログインにGoogle Identity ServicesのブラウザIDトークンを使う

## context

主催者の作成・編集・集計だけを認証し、回答者の匿名回答を維持する必要がある。Cloudflare Worker側にはGoogle IDトークンの署名・issuer・audience・nonce検証とHttpOnlyセッションが実装済みである。

## decision

主催者のブラウザログインはGoogle Identity ServicesのIDトークンをnonce付きでWorkerへ交換する。

## rejected options

- Authorization Code Flow: Calendar APIやrefresh tokenが不要な今回には状態管理と秘密情報が過剰になる。
- Cloudflare Access: アプリ全体の認証になり、匿名回答者の要件と合わない。
- Firebase/Auth0: 新しい認証基盤と運用設定を追加するため、既存Worker検証と重複する。

## consequences

Client IDは公開値として配信し、Client SecretはWorker Secretに限定する。GIS外部スクリプトへの依存とGoogle側の設定が増える。回答者向け認証や履歴は別の判断に分離できる。
