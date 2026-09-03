# ADR 0018: continuationの復旧とauthority表示境界を固める

Status: accepted

Date: 2026-09-02

Amends: [ADR 0005](0005-separate-public-event-access-and-organizer-capability.md)、[ADR 0017](0017-create-series-only-from-explicit-account-continuation.md)

## context

Story 0012の初回実装は、主催者が明示した場合だけseriesを作り、event、候補日時、membershipを一つのtransactionへ保存した。
独立レビューでは、この中心境界は保たれていたが、秘密のdebug表示、read-only planのlock、失敗後の復旧に不足が見つかった。

作成成功responseは、browserが主催者権限を保存するため、生の主催者capabilityを一度だけ含む。
この型が自動導出の `Debug` を持つと、現在はlogへ出していなくても、将来のtracing、panic、test失敗で復旧キーを出せる。
JSON responseに生値が必要なことと、開発者向けdebug表示に生値が必要なことは別である。

continuation planはread用途だが、sessionの期限判定と必要なlast-seen更新を同時に行うため、初回実装では全memberの検査まで `BEGIN IMMEDIATE` を保持した。
seriesが長くなるほど、匿名作成と回答が必要とするSQLite writerを待たせる。
createは保存直前にactive session、owner、current tailを同じwrite transactionで再検証するため、plan全体をwriterへ置かなくても原子性を保てる。

UIでは、409後に最新planを取得しても、利用者が編集中の名前を上書きしない方針を採った。
ただし最新の候補を別に示さなければ、古い `#2` をそのまま再送して二つ目の `#2` を作れる。
また、送信中にsessionが失効した場合、同じタブでlogin画面へ移ると保持したdraftを失う。
native `details` も、`summary` をflex layoutに変えるとbrowser既定のmarkerを失うため、展開できることを視覚だけで判断しにくくなる。

## decision

- 生の主催者capabilityを含む `CreatedEvent` はcustom `Debug` を実装し、event projectionは表示しても `organizer_capability` の値を常に `[REDACTED]` とする。`Serialize` は作成成功responseに必要なため維持する。
- continuation planは、既存のaccount履歴と同じ二段階にする。sessionの期限判定、必要なtouch、期限切れrow削除を短いwrite transactionでcommitし、その後にowner認可、series、member、current tailを一つのDEFERRED read transactionで検査する。
- continuation createは従来どおり `BEGIN IMMEDIATE` の中でsession、owner、tailを再検証する。planとcreateの間の変更はexpected tailの409で閉じ、plan readのsnapshotをwrite予約の代わりにしない。
- file-backed SQLiteで同じtailから二要求を開始し、成功一件とstale一件になり、seriesが分裂しないことを回帰testにする。
- series作成後またはevent作成後のcandidate保存失敗でも、series、event、candidate、membershipがすべてrollbackすることを回帰testにする。
- 空series、一memberだけのseries、消失event、owner不一致など、schema外から持ち込まれた破損を部分表示せずdata invariant違反にする境界を回帰testにする。
- 409後にplanを読み直しても、入力中の名前、ひとこと、候補日時を自動変更しない。最新tailから候補を作れる場合は、現在の入力とは別に値を表示し、「最新の候補を使う」という明示操作でだけ名前へ反映する。表示中の最新候補は、明示適用、作成成功、またはさらに新しい409で古くなるまで保持し、再送の401または一時的な失敗だけでは破棄しない。
- create requestまたはplan再読込の進行中は「最新の候補を使う」を無効にする。送信開始後に画面の名前だけを変え、確定済みrequestと成功表示を食い違わせない。新しい409を受けた状態では、過去のplanに候補があったという説明よりstale案内を優先する。
- 送信時の401ではdraftを同じcomponent stateへ残し、loginを別タブで開くlinkをerror領域に置く。login後は元のタブへ戻って再送できる。自動login遷移、匿名作成へのfallback、draftの永続保存は行わない。
- continuation formのclient validation失敗では、描画後に最初の該当fieldへfocusを移す。名前、主催者のひとこと、候補日時の順に、表示したerrorと入力位置を対応させる。
- series履歴の `details` と `summary` はnative semanticsを維持しつつ、flex layoutでも消えないmarkerと `:focus-visible` outlineをCSSで明示する。

参考資料:

- [Rust: `std::fmt::Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html)
- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html)
- [SQLite: Isolation](https://www.sqlite.org/isolation.html)
- [WHATWG HTML: The details element](https://html.spec.whatwg.org/multipage/interactive-elements.html#the-details-element)
- [WCAG 2.2: Focus Order](https://www.w3.org/WAI/WCAG22/Understanding/focus-order.html)
- [WCAG 2.2: Focus Visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html)

## rejected options

### `CreatedEvent` の自動 `Debug` を残す

実装は最小で済む。
しかし、bearer authorityを将来のlogへ偶発的に出せる型契約を残すため採用しない。

### planの全読込を `BEGIN IMMEDIATE` に置く

plan取得中にtailが変わらない。
しかし、plan responseを受け取ってからcreateするまでの変更は防げず、expected tail検査はどのみち必要である。
read中だけ匿名writeを止める利益よりlock時間が大きいため採用しない。

### 409後に名前を最新候補へ自動置換する

重複した連番を避けやすい。
しかし、利用者が連番ではない名前へ編集した意思まで消すため採用しない。

### 最新候補を表示せず、入力保持だけを優先する

UIは短くなる。
しかし、staleになった理由と新しい候補を比較できず、古い候補を再送しやすいため採用しない。

### 401で同じタブをloginへ移す

loginへの導線は単純になる。
しかし、永続化していないdraftを破棄するため採用しない。

### flexの `summary` でもbrowser既定markerへ任せる

追加CSSが不要になる。
しかし、layout指定によってmarkerが消えるbrowserがあり、展開可能性を見た目から判断できないため採用しない。

## consequences

- JSON success responseは主催者capabilityを一度だけ返せる一方、通常のdebug表示から生値を除ける。
- plan readはwriterを長く占有しない。session解決後にlogoutが競合したreadは完了し得るが、createはactive sessionを再検証するため新eventは保存しない。
- 409後は入力を守りながら最新候補も選べるが、利用者が古い名前を意図的に再送することまでは禁止しない。event名の一意性をseries identityにしない方針を維持するための残余リスクである。
- 別タブloginはdraftを守るが、tabを閉じる、再読込する、browser processが破棄される場合までは復旧できない。draft永続化は保存範囲を別Storyで決める必要がある。
- custom markerとfocus制御が増え、native elementだけに任せるよりCSSとUI testが増える。その代わり、320pxとkeyboardで操作の所在を見失いにくい。
- corruption testはforeign keyを意図的に無効化する場合がある。productionで許容する操作ではなく、repositoryがschemaだけに依存せずfail-closedになることの検証として限定する。
