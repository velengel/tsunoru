# ADR 0024: worktree作成前にGit fileのローカル実体化を要求する

## context

`git worktree add` はlinked worktreeごとに `HEAD` とindexを分けるが、Git objectなどはrepositoryで共有する。
そのため、作成先を `/private/tmp` にしても、`Documents` 配下の共通Git directoryを読み続ける。

このPCでは `Documents` がmacOS File Providerの管理対象だった。
TSUNORUの `.git/objects` には926ファイル中616ファイル、作業ツリーには24ファイルの `dataless` placeholderがあった。
停止中の別Git processをsampleすると、`git status` は `refresh_index` からworking fileを再計算する `index_fd` に入り、`mmap` で待っていた。
同じprocessが開いていたfileはiCloud管理下のrepositoryにあった。

CloudDocs全体の状態表示では、別repositoryの `.codex/worktree` を含む大量のitemが `needs-sync-up` として約49分滞留していた。
linked worktreeのGit管理状態はrepositoryごとに分かれていても、File Provider配下の作成先はPC全体の同じ同期queueへfileを追加する。

Gitはworktree作成中、共通Git directoryの `worktrees/<id>/locked` に `initializing` と書く。
Git sourceでは、不完全な管理情報がpruneされないようcheckout前に作り、準備完了後に削除する。
したがって `locked initializing` は単独では失敗の証拠にならない。

参考:

- [Git: git-worktree](https://git-scm.com/docs/git-worktree)
- [Git source: builtin/worktree.c](https://github.com/git/git/blob/master/builtin/worktree.c)
- [Apple: Work with folders and files in iCloud Drive](https://support.apple.com/guide/mac-help/work-with-folders-and-files-in-icloud-drive-mchl1a02d711/mac)
- [Apple: Store files in iCloud Drive on Mac](https://support.apple.com/guide/mac-help/store-files-in-icloud-drive-mchle5a61431/mac)

## decision

- TSUNORUのworktreeはrepository内scriptから作る。直接の `git worktree add` を通常入口にしない。
- macOSでは作成先とその既存ancestorへ `com.apple.file-provider-domain-id` があればexit code 75で拒否する。worktreeはiCloud DriveなどのFile Provider domain外へ作る。
- scriptはGit変更より先に、共通Git directoryのobjectと作成元worktreeをscanする。macOSで `dataless` fileがあればexit code 75で停止し、件数、例、Finderの「ダウンロードを保持」を示す。
- `dataless` の検出ではfile内容を開かない。`find -flags +dataless` でmetadataだけを読み、preflight自身が長いdownloadを始めないようにする。
- repository内の `.codex/worktree` はpreflightの除外対象にしない。既存のnested worktreeが残る場合も、そこにあるplaceholderを見逃さない。
- worktree作成は共通Git directory内のrepository単位lockで直列化する。lockにはPID、開始時刻、呼び出し元、branch、作成先を残す。
- lockが既にあれば後続の作成を始めない。PIDが見えない場合も自動削除せず、process、`git worktree list --porcelain`、lock内容を調べる。
- 作成は `git worktree add --quiet` を同期実行する。進捗出力を抑えるのはpipe詰まりを避けるためであり、checkout自体を省略する判断ではない。
- command runnerが実行session IDを返した場合、呼び出し側は同じsessionをpollする。JavaScript cellの終了表示を子processの終了と読み替えない。
- Gitが `locked initializing` を表示していても、対応するGit processまたはfile更新が続いていれば待つ。手動unlock、prune、force removeは行わない。
- 作成完了後に新しいworktreeのpath、branch、HEAD、clean statusとGit内部の `locked` file不在を検証する。
- Appleが案内する永続的な実体化操作はFinderの「ダウンロードを保持」とする。`brctl` やFile Provider属性の直接変更は公開された恒久interfaceとして扱わない。
- `isKeepDownloaded=1` は保持方針の確認にだけ使う。`isRecursivelyDownloaded=1` かつpreflightが0件になるまでは実体化完了と判定せず、Git操作を再開しない。内容読取による取得が進まない場合は、そのsessionだけを中断し、File Providerの外部状態を待つ。
- Keep Downloaded後にもplaceholderが再増加する場合、preflightを迂回しない。全linked worktree sessionを停止してからrepository本体をFile Provider domain外へ移す作業を別途計画する。
- Finderを使う調査では、操作前にwindow ID、target、bounds、情報・詳細panelを記録する。終了時は差分として増えたwindow、情報・詳細panel、sheetを閉じ、再利用した既存windowを元のtargetとboundsへ戻してから再棚卸しする。Finder自体の終了や無関係な既存windowのcloseは行わない。

## rejected options

### worktreeの作成先だけを `/private/tmp` にする

linked worktreeは共通Git directoryのobjectを読む。
今回も作成先は `/private/tmp` だったが、共通objectの実体化待ちは残ったため却下する。

ただし作成先をFile Provider配下に置かないこと自体は必要である。
共通Git directoryの実体化gateと、作成先のFile Provider gateを両方通す。

### `locked initializing` を見つけたらworktreeを削除して作り直す

このlockは作成中の管理情報をpruneから守るGitの状態である。
動作中processを残したまま削除すると、同じbranch、path、管理情報へ別のGit操作を重ねるため却下する。

### 長いcommandをyield時点で失敗と判定する

command runnerは子processが継続中なら実行session IDを返す。
今回もpoll後にexit code 0で完了したため、短いyieldを終了判定に使わない。

### `brctl download` を恒久入口にする

macOS 26の公開helpにdownload操作は現れず、directoryを指定した実測でも再帰的に実体化されなかった。
未文書化CLIへ開発フローを依存させない。

### 複数sessionのworktree作成を常に許す

Gitは別branchのlinked worktreeを表現できるが、このPCでは複数のcheckoutが同じFile Provider上のobject読取を重ねる。
作成だけを短時間直列化しても、完成後の各worktreeで行う独立開発は並行できるため却下する。

### Finderを終了して全windowをまとめて閉じる

今回の調査と無関係なwindowや情報panelまで失われる。
開始前との差分を記録し、作業で増えたUIだけを閉じる方針とする。

## consequences

- iCloud placeholderが残る状態では、新しいworktree作成が数分停止する前に理由付きで失敗する。
- File Provider配下の作成先は、branchやworktreeを作る前に拒否される。別sessionの大量checkoutをCloudDocs同期queueへ追加しない。
- 利用者はFinderでrepositoryを「ダウンロードを保持」にする必要がある。初回download時間とlocal disk使用量が増える。
- Keep Downloadedの設定直後に同期が完了するとは限らない。設定済みでもpreflightが停止を続ける場合があるため、外部同期の待ち時間を受け入れる。
- repository本体のdomain外移行はlinked worktreeの共通Git directory pathへ影響する。稼働中sessionがある状態で自動移動しないため、今回の変更は未commitのまま安全なlocal worktreeに保持される可能性を受け入れる。
- 複数sessionはworktree作成だけを同時に行えない。作成後の編集、test、buildは別worktreeで並行できる。
- repository単位lockとGitのworktree lockは意味が異なる。前者はCodex間の作成直列化、後者はGit管理情報のprune防止である。
- 強制終了後のstale lockには人の確認が必要になる。安全側で停止するため、正常な作成を一時的に妨げる可能性を受け入れる。
- `dataless` がない他OSとローカルfilesystemではpreflightの追加時間だけが発生する。
- File Provider domain属性はmacOS固有である。他OSでは作成先を機械的に判定せず、利用環境側で同期filesystemを避ける必要がある。
- Finder操作には開始前と終了後の棚卸しが増える。一方で、調査用windowや情報panelを利用者のdesktopへ残さず、既存のFinder作業も壊さない。
