# ADR 0025: 必要な現行treeからpublicな履歴なしrepositoryを作る

## Status

Accepted

## context

TSUNORUのGit repositoryはiCloud Drive配下にあり、過去にGit objectと作業fileが`dataless`になった。
linked worktreeの作成先を`/private/tmp`へ変えても、共通Git directoryはiCloud側に残るため、Git操作の停止を解消できなかった。

現在はpreflightが`dataless` 0件でPASSし、Git lockとTSUNORU関連processも存在しない。
この状態なら、現行treeとlocal-only dataをFile Provider外へ読み出せる。

GitHubの`velengel/tsunoru`はpublicであり、まだdefault branchを持たない。
既存117commitは一つの個人emailをauthorとcommitter metadataに持つ。
高確度のcredential、private key、個人absolute path、credentialらしいtracked file名は検出されなかったが、個人emailを含む全履歴をポートフォリオ公開へ持ち込む必要はない。

main以外にも、完成済みのADR簡潔化、未commitのworktree停止予防策、未完了のカレンダー修正がある。
現在のmainだけをsnapshotにすると、必要な運用変更と作業中の機能を失う。

## decision

TSUNORUは、必要な現行treeを状態別に統合してGitHub noreply emailの履歴なしmainとfeature branchへ再構成し、public GitHubからFile Provider外へfresh cloneする。

## rejected options

### 既存Git履歴と全branchをpublicへpushする

commit単位の経緯を残せるが、117commitのmetadataに個人emailが含まれる。
利用者はStoryとADRを判断の正本とし、履歴を捨てることを受け入れたため採用しない。

### private GitHubへ既存履歴を保存する

個人emailを公開せず、既存commitとbranchをそのまま復元できる。
しかし、TSUNORUをポートフォリオとしてpublicにする目的を満たさない。

### 現在のmainだけを履歴なしsnapshotにする

もっとも単純だが、未統合のADR簡潔化、worktree停止予防策、カレンダー修正を失う。
必要な作業を状態別に残すという利用者の方針に合わない。

### 未完了のカレンダー修正をmain snapshotへ統合する

一つのtreeだけで公開できるが、Storyの未完了項目とbrowser検証の不足を隠す。
mainは完成済みの変更に限定し、カレンダー修正はfeature branchへ置く。

### `.git`を含むrepository全体をコピーする

worktree関係と履歴を保てるが、File Providerに置かれていたGit directoryを新しい正本へそのまま持ち込み、fresh cloneによる復元可能性を証明できない。
29 GiBの`target/`まで複製しやすいため採用しない。

### local SQLiteをrepositoryへcommitする

新配置だけでdataを復元できるが、accountとeventのdataをpublicにする。
DBはGit管理外のlocal stateとしてchecksum付きで複製する。

## consequences

- public remoteには、個人emailを含む既存117commitが入らない。
- mainには完成済みのsourceと運用変更を置き、未完了のカレンダー修正はfeature branchから継続できる。
- StoryとADRの番号競合を解消する編集が必要になる。
- GitHub上のcommit時系列は移行後から始まり、過去の実装順序を示さない。
- SQLiteとignored quizはGitHubから復元できない。新配置へ別経路で複製し、checksumをmanifestへ残す。
- `target/`を再生成するため、初回testとbuildに時間がかかる。
- fresh cloneとrepository内worktreeを検証した後も、旧iCloud sourceは自動削除しない。
- `LICENSE`は追加しない。public repositoryでsourceを見せることと、第三者へ利用を許諾することは別に扱う。
