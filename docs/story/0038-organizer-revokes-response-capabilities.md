# Story 0038: 主催者が回答 capability を失効させる

## context

Issue #27。共有回答 URL や回答 capability が漏れた場合に、主催者が対象回答だけを止められるようにする。公開イベントの閲覧と新規回答は維持し、失効した回答の再送・編集だけを拒否する。

## definition of done

- 主催者 capability で対象イベントの回答 capability を失効できる。
- 主催者以外、別イベント、存在しない回答は変更できない。
- 失効後の同じ回答 capability による再送・編集は拒否される。
- 公開イベントの閲覧と新しい回答 capability による匿名回答は継続できる。
- capability の平文を保存・応答・ログへ出さない。
- disposable D1/Worker で失効、認可、再送拒否、新規回答、競合を検証する。

## to do

- [x] 回答失効列と主催者認可 endpoint を追加する
- [x] 失効後の回答操作と競合のテストを追加する
- [x] Story、ADR、README、検証記録を実装へ同期する

## concern

主催者自身の capability を失効させるには別の認証主体が必要になるため、今回は対象回答の失効に限定する。主催者 capability の交換は別判断とする。失効は公開イベントや新規匿名回答を止めない。
