# Story 0018: iCloud未実体化によるGit停止を作業前に防ぐ

Status: in progress

Date: 2026-09-02

## context

複数のCodex sessionがTSUNORUで別々のworktreeを作った際、二つの `git worktree add` が数分間継続した。
Gitは作成中のlinked worktreeを `locked initializing` と表示したが、これは失敗ではなく、checkout完了前の管理情報をpruneから守る内部状態だった。

同時実行だけでは遅延を説明できない。
TSUNORUはiCloud Driveの対象である `Documents` 配下にあり、調査時点で `.git/objects` の926ファイル中616ファイルと、作業ツリーの24ファイルにmacOSの `dataless` flagが付いていた。
465 byteのGit objectを一つ読むだけでも、iCloudからの実体化に約30秒かかった。

worktreeを `/private/tmp` に置いても、linked worktreeはmain worktreeのGit objectを共有する。
作成先だけをiCloud Driveの外へ出しても、共通Git directoryが未実体化なら停止を防げない。

PC全体のCloudDocs状態には、別repositoryの `.codex/worktree` を含む大量のitemが約49分前から `needs-sync-up` として滞留していた。
別sessionのworktreeはGitのHEADやindexを直接競合させていなくても、File Provider配下へ作れば同じ同期queueへ大量のfileを追加する。

## definition of done

- worktree作成前に、共通Git directoryと作成元worktreeの `dataless` fileを読み込まずに検出する。
- `dataless` fileが一件でもあれば、Gitの変更操作を始めず、Finderの「ダウンロードを保持」を案内して終了する。
- Finderで保持を指定済みでも、preflightが0件になるまでは実体化完了と扱わない。進捗のない読取sessionは中断し、Git操作を重ねない。
- 複数sessionが同じrepositoryでworktree作成を始めた場合、一件だけが進み、後続は実行中の所有情報を示して終了する。
- worktree作成はrepository内の専用scriptを入口とし、branch、絶対path、開始commitを固定して実行する。
- 作成先の既存ancestorにFile Provider domain属性があれば、worktreeとbranchを作る前に拒否する。repository内の `.codex/worktree` もscan対象から除外しない。
- `git worktree add` が長時間継続した場合、呼び出し側は実行sessionをpollし、短いyieldを失敗と扱わない。
- `locked initializing` を手動unlockまたはforce removeせず、Git processとfile更新を確認して待つ。
- 作成後にpath、branch、HEAD、status、`locked` file不在を検証する。
- Finderを使う前に既存window、target、bounds、情報・詳細panelを記録し、作業後は今回増やしたUIを閉じる。既存windowを再利用した場合は元の状態へ戻し、再棚卸しする。
- 公式資料、PC上の観測、推測を分けた検証記録を残す。
- scriptの利用者向け失敗を先にtestし、実装後に回帰testを通す。

## to do

- [x] Git公式文書とGit sourceでlinked worktree、共有範囲、`--no-checkout`、`locked initializing` を確認する。
- [x] PC上のGit process、File Provider属性、`dataless`件数、停止中stack、open fileを確認する。
- [x] worktree作成とローカル実体化の運用判断をADRへ記録する。
- [x] materialization未完了と同時作成を再現する失敗testを書く。
- [x] ローカル実体化preflightとworktree作成lockを実装する。
- [x] File Provider配下の作成先を拒否し、既存のrepository内worktreeもpreflight対象にする。
- [x] AGENTS.md、README、用語集へ入口と復旧境界を反映する。
- [x] 調査で増やしたFinder windowを閉じ、既存UIを保持したことを確認する。
- [x] Finder調査の開始前記録と終了後cleanupを共有ルールと運用testへ追加する。
- [x] focused test、syntax、秘密情報検査を確認する。
- [ ] preflightがPASSした後、Git状態を確認してcommitする。
- [x] 検証記録とSurprise & Discoveryを更新する。

## concern

- `dataless` はmacOS File Provider固有であり、他OSでは同じ検査を実行できない。非macOSでは対象外として明示する。
- File Provider domainの検出にはmacOSの `com.apple.file-provider-domain-id` metadataを使う。内容は変更せず存在だけを見るが、macOS固有のため他OSでは作成先gateを適用できない。
- Finderの「ダウンロードを保持」はAppleが案内する永続操作だが、command lineから安定して設定する公開interfaceは確認できなかった。scriptは未文書化属性を書き換えない。
- `isKeepDownloaded=1` でも、同期中またはFile Provider待ちでは `isRecursivelyDownloaded=0` と `dataless` fileが残り得る。設定値ではなくpreflightの0件を完了条件にする。
- Finderは既存windowを再利用する場合がある。IDだけで一律に閉じず、開始前との差分で新規UIを閉じ、再利用した既存windowはtargetとboundsを復元する。
- repository全体をローカルへ保持するとdisk使用量が増える。対象は開発repositoryに限定し、利用者がFinderで解除できる状態を保つ。
- 作成lockを導入しても、scriptを経由しない直接の `git worktree add` は防げない。AGENTS.mdを共有入口にする必要がある。
- processが強制終了すると作成lockが残る可能性がある。自動削除はせず、PIDとworktree管理情報を調べてから復旧する。
- Keep Downloaded後にもplaceholderが再増加した場合、repository本体をFile Provider外へ移す必要がある。稼働中のlinked worktreeが参照する共通Git directoryを無断で移動せず、全sessionを止めてpath移行を別作業として行う。
