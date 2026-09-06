# ADR 0056: 試用コードを期限付きの Cookie に交換する

## context

前段の Bearer API は手動検証向けであり、共有鍵を bundle に埋めると公開した時点で入口制限がなくなる。既存 account の OAuth は Workers/D1 操作に使えるが、Access の設定済み運用は既存2アプリから確認できていない。限定版のためだけに Apple account や新しい Zero Trust 組織を導入する必要性は低い。

## decision

試用コードを同一 origin の POST で検証し、HMAC-SHA256 で署名した12時間の HttpOnly・Secure・SameSite=Strict・host-only Cookie に交換する。

## rejected options

- 共有 Bearer を bundle、共有 URL、localStorage に保存する。漏えい先や保存期間を増やす。
- Cookie の値を期限だけで判定する。ブラウザー側で改ざんできる。
- 全利用者の account / OAuth を同時に移植する。今回の限定試用に不要な管理対象を増やす。
- 今回のために Cloudflare Access を新規構築する。別の権限設定が必要で、Static Assets 付き Worker には `ctx.access` が届かない制限もある。需要が生じた段階で再判断する。

## consequences

これは本人確認ではなく試用の入口であり、主催者・回答 capability は別に必要である。署名には既存のランダムな `STAGING_API_TOKEN` を用い、値と署名を応答本文・ログへ出さない。コード入力成功後は入力値を破棄する。環境 secret の交換で既存 Cookie も無効になるが、個別失効は行わない。

退場はそのブラウザーの Cookie を削除する操作であり、コピー済み Cookie の個別失効は保証しない。コピーは12時間の期限または secret 交換まで有効となる。

Cookie による変更要求と入退場 POST/DELETE は完全一致 Origin を必須とする。従来の明示 Bearer による CLI は維持する。改ざん、期限切れ、異なる Origin、設定不備を DB 操作前に拒否する。長いランダムコードの配布が必要という使い勝手の制約を受け入れる。一般公開や利用者別の管理は #12 で判断する。

根拠: [Cookie attributes](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie)、[RustCrypto HMAC](https://docs.rs/hmac/0.12.1/hmac/)、[Access limitations](https://developers.cloudflare.com/workers/configuration/cloudflare-access/#ctxaccess-limitations)。
