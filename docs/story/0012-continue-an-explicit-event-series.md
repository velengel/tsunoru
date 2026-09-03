# Story 0012: 同じ活動だと選んだイベントを続けられる

Status: in progress

Date: 2026-09-02

Product sequence: Story 10

## context

Product Story 8と9で、login中に主催したeventは履歴へ残り、調整時の痕跡まで振り返れるようになった。
同じ活動を何度もつのる場合も、現在は過去の名前を見て、新規作成画面へ戻り、名前を手で入力し直す必要がある。

`ベストユニゾン #1` のように末尾の連番が明示された名前なら、利用者が「同じ活動を続ける」と選んだ後に限り、次の名前を控えめに提案できる。
一方、名前が似ているだけのeventを自動でまとめたり、通常の「飲み会」に回数を付けたりすると、活動の意味をsystemが勝手に決めてしまう。

seriesは名前の一致ではなく、主催accountが明示した関係として扱う。
これにより、提案された名前を編集して表記が変わっても同じ活動の履歴を辿れ、初回eventや単発eventへseries登録の作業を要求しない。

## definition of done

- login中に主催したeventのprivateな履歴詳細からだけ、「同じ活動の次回をつのる」を明示的に開始できる。
- loginしていない利用者、回答しただけの利用者、別accountの利用者は、そのeventをseriesの起点にできない。
- 通常の新規作成画面、履歴を開いただけの状態、public event、回答flowでは、series判定または次回名の提案を行わない。
- 起点または既存seriesの末尾名が、非空の名前と半角spaceを挟んだ末尾の `#N` に厳密に一致するときだけ、checkedな `N + 1` を次回名の候補として示す。
- `ベストユニゾン #1` には `ベストユニゾン #2` を提案し、連番のない `飲み会`、途中の数字、全角記号、先頭0付き、上限超過、長さ上限超過には名前を提案しない。
- 名前の候補はevent名inputへ編集可能な値として入り、提案である理由をtextで説明する。利用者は自由に変更できる。
- 名前を変更しても、明示したseries関係は名前の一致から独立して保存され、同じ活動の履歴にまとまる。
- 次回eventの本体、候補日時、accountの主催関係、seriesと新旧eventの関係は、一つのtransactionで全部成功または全部失敗する。
- continuation中にaccount sessionが失効した場合、単発のanonymous eventとして黙って保存せず、入力を残してloginが必要だと示す。
- 初めて次回を作るときだけ起点eventと新eventのseriesを作り、既存seriesから続ける場合は同じseriesへ安定した次の順番で追加する。
- 二つの並行したcontinuationでも、同じ起点を二つのseriesへ分裂させず、一eventを複数seriesへ所属させない。
- account履歴は明示されたseriesだけを「継続している活動」としてまとめ、series内を新しい回から過去へ辿れる。単発の主催履歴と参加履歴は従来どおり残す。
- account削除時はseries関係だけを削除し、public-by-linkなevent aggregateは従来どおり保持する。
- seriesの内部ID、account ID、session、capability、token、hashをbrowserへ返さず、public eventと匿名回答のprojectionを変えない。
- continuation画面はSSRへprivateな起点情報を埋め込まず、loading、未login、session期限切れ、missing、失敗、作成formを区別する。
- 320pxとdesktopで、長い起点名、提案理由、編集可能な名前、候補日時、失敗時の案内を読め、keyboardだけで通常作成へ戻れる。
- domain、migration、repository、server認可、UI、responsiveのtestを実装より先に追加し、期待した理由でREDになる。
- test、Clippy、format、Fullstack build、秘密情報検査が成功する。
- 8081の候補版を検証している間も、検証済みserverを別portで利用できる。

## to do

- [x] First Instructionの履歴、継続イベント、初期スコープ外との境界を確認する。
- [x] 現行のevent作成、account履歴、private履歴詳細、session認可を確認する。
- [x] Dioxus server function、SQLiteの複合foreign keyとtransaction、accessibleな入力補助を一次情報から確認する。
- [x] 明示的なseries関係、保守的な命名規則、account認可、失効時のatomicityをADRへ記録する。
- [x] 命名規則とcontinuation inputの失敗するdomain testを書く。
- [x] series schema、account削除、並行continuation、履歴groupingの失敗するrepository testを書く。
- [x] private API、SSR、cookie、public/anonymous境界の失敗するserver testを書く。
- [x] continuation各state、編集可能な提案、grouped history、responsiveの失敗するUI testを書く。
- [x] migration、domain projection、atomicなcontinuation repositoryを実装する。
- [x] private continuation planとcreate server functionを実装する。
- [x] 履歴詳細からcontinuation formへ進み、series履歴を辿れるmobile-first UIを実装する。
- [x] 通常作成、public event、匿名回答、既存履歴詳細が変わらないことを確認する。
- [ ] 320pxとdesktopの実ブラウザーで各state、名前編集、長い内容、keyboard操作を確認する。
- [x] 独立レビューを反映し、README、Story、対応表、検証記録、Surprise & Discoveryを更新する。

## concern

- 名前の類似だけでseriesを作ると、別の活動を誤ってまとめ、後からほどく操作が必要になる。
- `#N` のsyntaxだけで通常画面から提案すると、連番を望まない `飲み会 #17` に不自然な次回名を押し付ける。
- 名前をseriesのidentityにすると、表記揺れを許した瞬間に履歴が分裂する。
- series選択を初回作成formへ足すと、短いanonymous flowへ新しい分類作業を持ち込む。
- session失効時に既存のanonymous作成へfallbackすると、利用者はgrouped historyへ入ったと思ったeventを見失う。
- event保存後にseries関係だけ別transactionで書くと、作成成功なのに履歴へまとまらない部分成功が起きる。
- account-privateなseries情報をpublic projectionまたはSSRへ加えると、活動の関係を共有URLやcacheへ広げる。
- 連番の形式を広く受理しすぎると誤提案が増える。初期実装は厳密な `名前 #N` だけに絞り、表記揺れは名前parserではなく明示的なmembershipで受け止める。
