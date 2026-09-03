# Story 0008: 主催者が候補日時を一つ選んで確定する

Status: in progress

Date: 2026-09-02

Product sequence: Story 6

## context

Product Story 4と5で、主催者は候補ごとの回答サマリーと、回答者ごとの集計表を確認できるようになる。
これらは判断材料であり、どの候補を採用するかは決めない。

集まりの日時を決める最後のハンドルは、主催者に残す。
主催者が一つの候補を意識して選び、確定した事実を永続化できるようにする。
決定日時を回答者へ見せ、共有し、カレンダーへ持ち帰る体験はProduct Story 7へ分ける。

## definition of done

- 主催者用回答サマリーで、作成時の順序を保った候補日時から一つを選べる。
- systemは候補を事前選択せず、件数、判断補助、score、多数決から自動確定しない。
- 主催者が選んだ日時を確認できる文脈と、明示的な確定操作を分けて見せる。
- 正しい主催者capabilityを持つprivateなPOSTだけが日程を確定できる。
- event不在、誤ったcapability、別eventのcapabilityでは、event、候補、回答、決定状態を推測できる差を返さない。
- requestのcandidateが対象eventに属することを、保存transactionの中で確認する。
- 確定したcandidateと確定日時をSQLiteへ一つのaggregateとして保存し、server再起動後も残す。
- 同じ操作の通信再試行は二重決定を作らず、安全に同じ成功結果へ戻れる。
- 確定後のprivate画面は、選ばれた日時を明確に表示し、未確定の操作と混同させない。
- 読み込みまたは保存に失敗しても、表示中のサマリー、集計表、利用者が選んだ候補を失わず再試行できる。
- 320pxとdesktopで、候補の意味、選択状態、確定操作、失敗、成功をkeyboardから確認できる。
- 回答用共有画面への決定表示、共有control、iCalendar、外部calendar連携を追加しない。
- ログイン、profile、履歴、series、候補編集、推薦、投票ruleを追加しない。
- 利用者に見える受け入れtestと、認可、candidate所属、冪等性、競合、永続化のrepository testを実装より先に追加し、期待した理由でREDになる。
- test、Clippy、format、Fullstack build、秘密情報検査が成功する。
- 8081の候補版を検証している間も、検証済みserverを別portで利用できる。

## to do

- [x] First InstructionのProduct Story 6と、Story 5・7の境界を確認する。
- [x] 確定の変更可否、再試行と競合、保存aggregate、private projection、確認UIを一次情報から調査する。
- [x] 認可、transaction、冪等性、競合、確定後UI、Story 7非混入をADRへ記録する。
- [x] 明示選択、未選択、保存中、失敗、再試行、成功、keyboardの失敗する受け入れtestを書く。
- [x] 正しい認可、別event candidate拒否、同一再試行、異なる再決定、同時request、再起動後永続化の失敗するrepository testを書く。
- [x] 日程決定migration、domain、repository、private POSTを実装する。
- [x] サマリー上の候補選択、確認、確定後表示を実装する。
- [ ] 320pxとdesktopの実ブラウザーで、候補選択、失敗、再試行、成功、keyboardを確認する。
- [ ] 回答者contextでは決定を変更できず、URL、DOM、browser storage、logへ主催者capabilityの新しい露出がないことを実ブラウザーで確認する。
- [x] 独立レビューを反映し、README、Story、対応表、検証記録、Surprise & Discoveryを更新する。

## concern

- 「確定」が一度だけの不可逆操作か、主催者が後から変更できるかは、参加者への公開とcalendar持ち帰りの一貫性へ影響する。実装前に決める必要がある。
- 同じcandidateへの再試行と、異なるcandidateへの再決定は区別しないと、通信失敗時の安全性と意思変更を混同する。
- 二つのbrowser tabから同時に異なる候補を確定する競合は、最後の書き込みで黙って上書きしてはいけない。
- candidate IDは公開event projectionに既に含まれるが、決定requestでは対象eventへの所属を信頼せず再確認する必要がある。
- 回答0件でも主催者が日時を決める場合がある。回答件数を確定の必須条件にはせず、判断は主催者へ残す方向を検討する。
- 決定日時の公開前でも、private projectionとUIが異なるsnapshotを読めば古い未確定状態を見せる可能性がある。更新境界を明確にする必要がある。
- 確定時刻は監査と競合判断に使えるが、利用者のイベント開始日時と混同しない名前にする必要がある。
- 回答者への決定通知、共有、iCalendarはStory 7であり、保存の都合からこのStoryへ画面を先取りさせない。
