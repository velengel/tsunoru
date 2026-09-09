# ADR 0076: 主催者だけが回答 capability を個別失効させる

## context

回答 capability が漏えいしたとき、共有イベント全体を閉じずに対象回答だけを止める必要がある。主催者 capability は現在の操作認可そのものであり、同じ credential を自分で失効させる別認証主体は存在しない。

## decision

主催者 capability で対象回答を失効させ、失効後はその回答 capability の再送・編集だけを拒否する。

## rejected options

共有イベント全体の削除や staging token の交換は他の回答者を巻き込み、回答 capability の失効を名前・回答 IDだけで許可する案は認可を迂回するため退ける。主催者 capability 自身の失効は別の認証主体がないため今回採用しない。

## consequences

失効した回答は履歴として主催者の集計に残るが、同じ capability では更新できない。公開イベントの閲覧と新しい capability による匿名回答は継続する。`responses.revoked_at` の更新と認可確認が増え、主催者が誤失効した場合の復元操作は別途必要になる。
