# Story 0032: 主催者だけGoogleログインで利用する

## context

主催者が自分のイベントを継続的に管理できるよう、Cloudflare版へGoogleログインを追加する。回答者はログインなしで回答できる状態を維持する。認証基盤を先に整え、ダッシュボード・任意回答者履歴・コメント・候補おすすめ・表示整理・運用計装を後続の小さなStoryへ分ける。

## definition of done

- [ ] Googleログイン成功後、Workerがアプリ固有のHttpOnlyセッションを発行する。
- [ ] 主催者の作成・集計・削除APIがアプリセッションを要求する。
- [ ] 回答者は未ログインでイベント取得と回答送信を完了できる。
- [ ] `state`、nonce、issuer、audience、expiryを検証し、ID Tokenや認証情報をログ・URL・localStorageへ残さない。
- [ ] 認証失敗、未設定、異なるOriginを安全に拒否する。
- [ ] Google client設定をリポジトリへ保存しない。
- [ ] 失敗するHTTP・Worker統合テストを先に追加し、Rust/Worker検証を通す。
- [x] 実装前の理解ゲートを2問で完了する。3問を超える場合はPRを分割する。

## to do

- [x] OIDCの公式仕様と既存native認証・履歴境界を調査する。
- [x] ADRとStoryで主催者必須・回答者任意の境界を記録する。
- [x] [理解ゲート](../../.mydocs/260906-google-organizer-auth-understanding-gate.html)を作成する。
- [ ] OIDC開始、callback、ID Token検証、セッション発行を実装する。
- [ ] organizer API認可と匿名回答の回帰テストを追加する。
- [ ] Google Cloud Console設定とWorker Secret登録手順を文書化する。
- [ ] 実ブラウザーとstagingで主催者・匿名回答の両方を確認する。

## concern

Googleログインはアプリの主催者識別を提供するが、既存のevent capabilityを自動的に別端末へ移すとは限らない。回答者の履歴や匿名回答のclaimは別設計とし、名前一致だけで過去回答を紐付けない。Google Calendar連携や通知はこのStoryに含めない。
