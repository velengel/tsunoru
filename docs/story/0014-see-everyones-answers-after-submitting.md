# Story 0014: 回答を送ったあとにみんなの回答を見られる

Status: in progress

Date: 2026-09-02

## context

回答者は全候補へ○、△、×を送り終えても、現在は「回答を送りました」と任意のひとことだけが表示される。
ほかの人の都合は主催者用の集計表に存在するが、回答者には見えない。

回答後にみんなの回答を見られれば、自分の回答が一覧へ入ったことを確かめ、集まりやすそうな日を同じ画面で共有できる。
ただし、共有URLを開いただけの人へ回答者名と個別の都合を常時返す必要はない。

一覧を返す条件は、そのbrowserが一件の完全な回答を保存できたこととする。
回答POSTの成功responseに同じsnapshotの回答一覧を含めれば、別の読取secret、URL parameter、cookie、localStorageを増やさずに「送った後」を境界にできる。

## definition of done

- 回答を正常に保存した直後、同じ画面に回答者と全候補の○、△、×を一覧表示する。
- 一覧には今送った回答を含め、同名の別回答を黙ってまとめない。
- 回答POSTは、初回保存と同一payloadの安全な再試行のどちらでも完全な一覧を返す。
- 共有URLを開いただけの初期HTMLとpublic event取得には、回答者名または個別回答を追加しない。
- 一覧の取得だけを目的とした公開GET、URL parameter、cookie、localStorageを追加しない。
- 回答保存後に一覧読取だけ失敗した場合、同じcapabilityとpayloadの再試行で二重回答を作らず復旧できる。
- 回答一覧は一つのSQLite read snapshotから再構成し、欠損・重複・未知のcellがあれば部分表を返さない。
- 表はcaption、列見出し、行見出し、○・△・×の意味を持ち、320pxでは表の領域だけを横scrollできる。
- 任意のひとことを保存またはskipしても、取得済みの回答一覧は画面から消えない。
- repository、server return type、成功UI、公開境界、responsive contractのtestを実装より先に追加する。
- test、Clippy、format、Fullstack build、秘密情報検査が成功する。

## to do

- [x] 現行の回答保存、response capability、主催者用集計表、公開境界を確認する。
- [x] 保存成功responseでだけ一覧を返す認可とsnapshot方針をADRへ記録する。
- [x] 回答capabilityで一覧を読めることと、別capabilityを拒む失敗するrepository testを書く。
- [x] 回答POSTが一覧を返す失敗するserver contract testを書く。
- [x] 回答後一覧と公開HTML非回帰の失敗するUI testを書く。
- [x] 保存後のparticipant-authorized readとtyped responseを実装する。
- [x] 回答成功画面へ一覧を統合し、ひとこと操作後も保持する。
- [ ] 320pxとdesktopでtable scroll、見出し、長い回答者名を確認する。
- [x] README、Story、検証記録、Surprise & Discoveryを更新する。

## concern

- 回答者名と個別回答の可視範囲を広げる。共有URLを知るだけでは読めず、回答成功responseに限定する。
- 回答数には上限がないため、成功responseとDOMは回答数×候補数に比例する。
- 保存transactionへ一覧読取まで含めるとwriter lockを長く保持する。保存をcommitしてからread snapshotを開く。
- commit後のread失敗をHTTP失敗として返すと、利用者には保存成否が曖昧に見える。同一capabilityのretryを必ずidempotentに保つ。
- 表へコメントまで含めると、回答直後の追加コメントとsnapshotの順序が複雑になる。初期実装は名前と○、△、×に限る。
