# ADR 0069: rate limitと濫用計測を公開前条件にする

## context
共有staging鍵だけでは公開後の濫用を利用者単位で抑制できない。

## decision
rate limitと濫用計測を実装・検証するまで一般公開しない。

## rejected options
現状の共有鍵のまま公開する案は、漏えい時の制御と観測が不足するため退ける。

## consequences
公開は延期され、匿名導線を維持したまま制限値・計測項目・保持期間を別Issueで決める。
