# ADR 0071: D1 backup/restore演習を公開前条件にする

## context
限定stagingの合成データ検証は障害復旧能力を証明しない。

## decision
D1 backup/restoreと保持ポリシーの演習を完了するまで一般公開しない。

## rejected options
バックアップなしで公開する案は、誤削除や障害から復旧できないため退ける。

## consequences
復旧手順とRPO/RTOの検証が後続作業として必要になる。restore後は期限切れsessionを無効化し、削除済みデータを再削除し、漏えいcapabilityを交換する。backup鮮度と容量も監視する。
