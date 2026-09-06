# ADR 0051: staging API を明示的な Bearer で制限する

Status: accepted

## context

まだ UI やアカウントを移植していない API を限定利用者で試す。ブラウザーが自動送信する Cookie を使わず、staging 全体への入口とイベント・回答ごとの権限を別にする。Origin の確認だけでは CLI からの呼び出しを認証できない。

## decision

全 staging API に設定済みの Bearer token を要求し、Origin がある要求は設定した一つの origin との完全一致だけを許す。

## rejected options

- 直ちに account と Cookie session を移植する。現在の API 検証に不要なログイン画面・失効管理が増える。
- Origin またはイベント ID だけで認証する。非ブラウザーから指定できる値であり、秘密の所持を証明しない。
- 未設定時だけ認証を省略する。設定漏れで公開されるため採らない。

## consequences

鍵の未設定・不正設定は 503、認証失敗は 401 で DB 操作前に閉じる。64 桁の暗号学的乱数 hex を Wrangler secret として渡し、応答とログに出さない。Origin なしの CLI は鍵が正しければ許可する。CORS は許可しないため、将来の UI は同じ origin に配置するか別 ADR で扱う。共有鍵の失効は環境 secret の交換で行うが、利用者別の失効・一般公開用のレート制限は別作業である。

根拠: [Workers secrets](https://developers.cloudflare.com/workers/configuration/secrets/)、[Workers best practices](https://developers.cloudflare.com/workers/best-practices/workers-best-practices/)。
