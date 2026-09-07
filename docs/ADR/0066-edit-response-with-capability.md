# ADR 0066: response capabilityで保存済み匿名回答を更新する

## context
Issue #17では、匿名回答者が同じブラウザーから入力ミスや都合の変更を修正したい。公開event IDや回答者名は共有URLから得られ、認可情報ではない。既存のresponse capabilityは初回送信時に生成され、ブラウザーの保存領域へ保持されている。

## decision
同じresponse capabilityのhashで既存responseを認可し、未決定イベントに限り名前と全候補の回答を一つのtransactionで置き換える。

補足：更新はresponse capability、event ID、完全な候補集合の一致を必須とし、決定済みイベントは拒否する。レスポンスではcapabilityを返さない。

## rejected options
- 回答者名や公開event IDだけで更新する：第三者が他人の回答を変更できる。
- 新しいresponseを追加して旧回答を無効化する：集計に重複が残り、同じ回答者の履歴管理が別問題になる。
- capabilityをURLへ埋め込む：共有URL経由で秘密が拡散する。

## consequences
同じブラウザーでは編集できるが、capabilityを失った別端末からの復旧はできない。更新処理は既存行と回答行を同一transactionで置き換えるため、競合時は既存データを保持する。決定後の回答は不変のままになる。
