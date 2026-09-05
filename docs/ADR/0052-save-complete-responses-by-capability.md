# ADR 0052: 回答 capability ごとに全候補を一括保存する

Status: accepted

## context

#9 の共通鍵は参加者間の所有権を分離せず、単一候補の upsert では未回答候補が残る。native の `src/storage.rs` は回答 capability ごとの全候補保存、同内容再送の成功、変更内容の競合を区別する。この規則を D1 の batch に移す。

## decision

回答ごとの capability hash を一意キーとし、全候補の正規化した内容が一致する再送だけを D1 batch で受け入れる。

## rejected options

- 表示名や公開回答 ID を権限にする。同名参加者や ID を知った別人を区別できない。
- 共通鍵と upsert を続ける。他人の回答の上書きと変更再送を許す。
- 候補ごとに独立 commit する。途中失敗で一部だけ保存される。

## consequences

回答者は最初の送信前に64桁の乱数 hex の capability を生成・保持する。D1 は hash だけを保存し、回答 ID を発行する。同名の別人は別 capability で回答できる。全候補を一度ずつ含むことを Worker と batch 内の条件で確認し、同時再送でも最初の内容を保持する。回答編集・失効・既存実験データの変換は含めない。

根拠: `src/storage.rs` の `record_availability_response_for_session`、[D1 batch](https://developers.cloudflare.com/d1/worker-api/d1-database/#batch)。
