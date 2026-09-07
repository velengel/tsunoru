# Story 0037: 一般公開前のrate limitとabuse対策

## context
Issue #26。共有staging鍵と匿名回答導線を保ったまま、Workerの作成・回答・読取を濫用から守る。

## definition of done
- 入口ごとのwindow、閾値、429契約をADRへ記録する。
- 送信元識別子はハッシュ化し、生のIP・capability・tokenを保存しない。
- disposable D1/Workerで正常系、超過、windowリセット、並行要求を検証する。
- 既存の認証・Origin・320px匿名回答導線を壊さない。

## to do
- [x] rate limitテーブルとWorker判定を実装する
- [x] 超過・windowリセット・ハッシュ保存の disposable テストを追加する
- [ ] 公開前の閾値見直しと実URL検証を行う

## concern
IP共有環境では正当な利用者を巻き込む。初期値は安全側の運用値であり、実測後に再調整する。
