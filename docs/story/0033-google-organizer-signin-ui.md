# Story: 主催者GoogleサインインUI

## context

Cloudflare版では主催者の作成・集計操作だけをGoogle認証で保護し、共有URLを受け取った回答者はログインなしで回答できる導線が必要である。

## definition of done

- 共有イベントURLは主催者セッションなしで開ける。
- ルートと集計画面はGoogle Identity ServicesのIDトークン交換後に表示される。
- nonceをブラウザーで生成し、サーバーへ送信する。
- ログアウトで主催者セッションを破棄できる。
- 320pxとキーボード操作を確認できる。

## to do

- [x] GISボタンとnonce交換を実装する
- [x] 公開回答URLと主催者画面のアクセス判定を分ける
- [x] CSPとAPIクライアントを更新する
- [ ] テスト・ビルド・ブラウザー確認を実行する

## concern

Google Client IDは公開値としてクライアントに含める。Client SecretやIDトークン検証はWorkerへ置き、回答者の認証・履歴は別Issueで扱う。
