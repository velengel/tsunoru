# Story 0019: TSUNORUをFile Provider外へ移す

Status: complete

Date: 2026-09-03

## context

TSUNORUの正本はiCloud Driveが管理する`Documents`配下にある。
過去にはGit objectと作業fileが`dataless`になり、worktree作成や`git status`がFile Providerの取得待ちで停止した。
現在のpreflightは`dataless` 0件でPASSしているが、Keep Downloaded後にもplaceholderが戻った実績があり、並列開発の正本として同じ場所を使い続けない。

GitHubには空のpublic repositoryである`velengel/tsunoru`がある。
TSUNORUは個人で使うアプリであり、同時にエンジニアとしての実績を示すポートフォリオでもあるため、現行sourceと判断記録をpublicにする。

既存117commitのauthorとcommitter metadataには個人emailが含まれる。
利用者はcommit履歴を捨て、StoryとADRを判断の正本として残すことを受け入れた。
public remoteには既存履歴をpushせず、必要な現行treeからGitHub noreply emailの新しいroot commitを作る。

## definition of done

- 完成済みのmain、ADR簡潔化、worktree停止予防策を一つの公開用main treeへ統合する。
- branch間で重複したStoryとADRの番号を、判断の内容を変えずに一意へ直す。
- 未完了のカレンダー修正はmainへ完成扱いで混ぜず、新しい`fix/calendar-layout-verification` branchとして復元する。
- 既存117commitと個人emailをpublic remoteへpushしない。
- public snapshotをcredential、個人path、個人email、local dataのpatternで検査する。
- `target/`、`var/`、`.mydocs/`、superseded frontend artifactをGit履歴へ含めない。
- `var/tsunoru.sqlite3`と`.mydocs/tsunoru-design-domain-quiz.html`をchecksum付きで新配置へ複製する。
- `target/`は複製せず、新配置のcommandから再生成する。
- 新しい正本を`$HOME/Developer/active/tsunoru`とし、GitHubからfresh cloneしてHEAD、tree、remote、local stateを照合する。
- repository local Git emailをGitHub noreplyへ固定する。
- repository内の`.codex/worktree/`でfeature worktreeを作成でき、作成元と異なるHEAD、index、build出力を使えることを確認する。
- mainと復元したカレンダーbranchについて、状態に合ったtestとbuildを実行し、未検証境界を分けて記録する。
- Codexのtrusted project pathを新配置へ切り替える前にbackupを作る。
- 旧iCloud repositoryと既存worktreeは、新配置の検証後も削除せず、別の承認までrollback元として保持する。

## to do

- [x] File Provider属性、Git状態、worktree、branch、process、容量を読取だけで棚卸しする。
- [x] 全履歴の個人email、credential pattern、個人path、tracked file名を検査する。
- [x] local SQLiteとignored成果物を、内容を公開せずに分類する。
- [x] Understanding Gateを`Passed`にする。
- [x] ADR簡潔化とworktree停止予防策を公開用main treeへ統合する。
- [x] StoryとADRの番号競合を解消する。
- [x] main snapshotのtest、build、差分、credential patternを検証する。
- [x] noreply emailのroot commitを作り、空のpublic GitHub repositoryへpushする。
- [x] fresh cloneを作り、local SQLiteとquizをGit管理外で復元する。
- [x] 未完了のカレンダー修正branchとrepository内worktreeを復元する。
- [x] Codex設定をbackupし、trusted project pathを切り替える。
- [x] manifest、検証記録、Surprise & Discoveryを更新する。
- [x] 旧iCloud repositoryを保持したまま、新配置を正本として利用できることを確認する。

## concern

- public snapshotにはsource、Story、ADR、検証記録が含まれる。既存履歴を除外しても、現行tree自体の公開検査が必要である。
- history-free snapshotではcommit単位の過去経緯と、branchが分岐した時系列を失う。
- StoryとADRは判断を残すが、すべての実装過程を再現するものではない。
- local SQLiteにはaccountとeventのdataがある。Gitへ追加せず、停止中のDBを整合した組として複製する。
- 未完了のカレンダー修正はcodeを含む。mainへ統合すると完成状態を誤認するため、公開remoteでもfeature branchに分ける。
- repository内worktreeを使うには、そのdirectoryをGitのscan対象から外し、作成後もlocal Git materialization gateを通す必要がある。
- public化だけではOSSの利用許諾にならない。現在の`LICENSE`なしという状態は変えず、第三者への利用許諾は別Storyで判断する。
- 旧sourceを削除するまでは二つのcopyが存在する。Codex設定と作業pathを切り替えた後は、新旧のどちらが正本かをmanifestで固定する。

## Understanding Gate（実装前理解確認）

- Status: `Passed`
- Reason: public remote、公開履歴、未統合作業、local data、正本path、rollbackを同時に決めるため。
- Questions: TSUNORUで将来まで残すものとsourceを見せる相手、privateで履歴を保つ案とpublicな履歴なしsnapshotのどちらを選ぶかを問うた。
- User explanation: TSUNORUは個人用だが、エンジニアのポートフォリオとして開発実績に使うためpublicにする。Git履歴は捨ててもよく、判断はStoryとADRから追える状態を優先する。
- Misalignment / Resolution: publicな全履歴は個人emailを公開するため採用しない。未統合作業は捨てず、完成済みの運用変更と未完成の機能branchを分けてhistory-free remoteへ再構成する。
- Unresolved: 旧iCloud sourceの削除時期と、第三者へ利用を許す`LICENSE`の要否は、この移行完了後に別途判断する。
