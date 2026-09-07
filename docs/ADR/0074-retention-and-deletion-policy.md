# ADR 0074: 保持・削除運用を公開前条件にする

## context
実データを扱うには、保持期間、削除担当、capability認証、restore後の削除再適用が必要である。

## decision
担当者・連絡先・保持期間・capability検証付き削除手順・restore後の再適用を定義するまで実データを受け付けない。

## rejected options
保持・削除運用を決めずに公開する案は、削除要求と復旧時の再発防止を保証できないため退ける。

## consequences
公開前に運用担当と手順の演習が必要になり、限定stagingは合成データに限る。
