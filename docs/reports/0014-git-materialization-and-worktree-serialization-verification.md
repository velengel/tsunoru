# Story 0018 検証記録

Date: 2026-09-02

## 結論

worktree作成の長時間停止には、二つの事象が重なっていた。
複数sessionが別々の `git worktree add` を同時に実行していたが、Gitは異なるlinked worktreeごとにindexとHEADを分ける。
同時実行だけを破損原因とは断定できない。

このPCでは、iCloud Drive管理下のGit objectが `dataless` placeholderになっていた。
Git objectを一つ実際に読むと約30秒待ち、読取後に `dataless` flagが消えた。
worktree作成先を `/private/tmp` にしても、共通Git directoryは `Documents` 配下に残るため、この待ちは残る。

repository内preflightは、現在のTSUNORUを0.1秒未満で `BLOCKED` にした。
同時作成testでは先行一件だけが成功し、後続は作成前にexit code 75で停止した。
syntheticなFile Provider domainを使うtestでは、作成先をbranch作成前にexit code 75で拒否した。

調査中に `open -R` で増えたTSUNORUのFinder windowは3枚だった。
window IDとtargetで対象を限定して閉じ、再棚卸しでは利用者のDownloads windowと二つのMP3情報windowだけが残り、TSUNORUのwindowとsheetは0件だった。

「今すぐダウンロード」の有無を追加確認した際は、TSUNORU windowが一枚増え、既存のDownloads windowもDocumentsへ再利用された。
新規windowを閉じ、既存windowのtargetとboundsをDownloadsへ戻した。
最終棚卸しはDownloads一枚と既存のMP3情報window二枚だけで、TSUNORU windowとsheetは0件だった。

## Git公式仕様

[git-worktree](https://git-scm.com/docs/git-worktree)は、linked worktreeが同じrepositoryへ結び付き、`HEAD` とindexなどのworktree固有fileを除いて共有すると説明する。
`git worktree add` は既定でcommitをcheckoutし、`--no-checkout` なら作成とcheckoutを分けられる。

[Git sourceの `builtin/worktree.c`](https://github.com/git/git/blob/master/builtin/worktree.c)は、準備中の管理directoryをpruneから守るため `locked` fileへ `initializing` と書き、準備完了後に削除する。
このため `locked initializing` は、作成processやfile更新が続く間は進行中の証拠である。

## Apple公式仕様

[AppleのiCloud Drive操作ガイド](https://support.apple.com/guide/mac-help/work-with-folders-and-files-in-icloud-drive-mchl1a02d711/mac)は、未download itemを「Download Now」で取得でき、特定のfileまたはfolderを「Keep Downloaded」にすれば、Optimize Mac Storageが有効でもMacへ保持できると説明する。

[AppleのiCloud Drive設定ガイド](https://support.apple.com/guide/mac-help/store-files-in-icloud-drive-mchle5a61431/mac)は、DesktopとDocumentsをiCloud Driveへ置けること、Optimize Mac Storageが容量不足時に古いdocumentをiCloudへ移すことを説明する。
このPCの `Documents` にはFile Provider domain属性があり、TSUNORUはその配下にある。

[Apple System Status](https://www.apple.com/jp/support/systemstatus/)は調査時点でiCloud Driveを利用可能と表示していた。
したがって広域障害より、このPCのFile Provider item解決または未完了syncを優先して疑う。ただしSystem Statusは個別accountや端末の正常性を保証しない。

## PC上の観測

### File Provider状態

初回計測は次の結果だった。

```text
dataless_objects=616
total_object_files=926
dataless_worktree_files=24
```

24件にはbuild成果物とlocal DBも含む。
preflightがGit対象としてscanする共通objectと、`target`、`var` を除く作業fileへ絞った後の観測は次のとおりだった。

```text
local_git_materialization=BLOCKED
dataless_git_objects=612
dataless_worktree_files=6
recovery=keep_downloaded_required
```

件数が616から612へ減ったのは、調査中のGit読取が一部objectを実体化したためである。
全fileが保持された証拠にはならない。

その後の再計測では、Git objectが749件、`target` と `var` を除く作業fileが77件へ増えた。
`fileproviderctl evaluate` もTSUNORU folderを `isDownloaded=1`、`isRecursivelyDownloaded=0`、`isKeepDownloaded=0` と報告した。
folder自体がdownload済みでも、子孫の保持は保証されない。

Finderで「ダウンロードを保持」を指定した後、`isKeepDownloaded=1` へ変わり、`dataless` はGit object 746件から121件、作業file 62件から24件まで減った。
その後は30秒以上変化せず、File Providerは `isDownloadRequested=0`、`isDownloading=0`、`isUploading=1`、`isRecursivelyDownloaded=0` を返した。

残ったfileを内容読取で実体化するsessionも90秒間件数が変わらなかったため、そのsessionだけをinterruptしてexit code 130で終了した。
この観測から、`isKeepDownloaded=1` は保持方針が設定された証拠にはなるが、再帰download完了の証拠にはならない。

最終のpreflightではGit objectが141件、作業fileが45件へ再増加した。
Keep Downloaded後にもplaceholderが戻る現在のPC状態では、repository本体をFile Provider外へ移すまで安定したGit再開を保証できない。

### 停止中process

別sessionの `git status --porcelain` は30秒以上継続していた。
1秒のprocess sampleは、全sampleで次のstackを示した。

```text
cmd_status
  refresh_index
    refresh_cache_ent
      index_fd
        xmmap
          mmap
```

`lsof` では、そのprocessがiCloud管理下にある別repositoryのworking fileを開いていた。
この観測はPC上のFile Provider待ちがTSUNORUだけの事象ではないことを示すが、その別repositoryの全遅延原因までは断定しない。

### CloudDocs全体の同期queue

Apple付属の `brctl status` は約2.4 MBを超える状態を返し続けたため、実行sessionをinterruptしてexit code 130で終了した。
得られた範囲では、CloudDocs clientが `needs-sync`、syncが `needs-sync-up` であり、別repositoryの `.codex/worktree`、Git object、source fileを含む多数のitemが約49分前から `pending-sync-up` または `sync-up-scheduled` のまま `next: ready` になっていた。

これは別sessionのworktreeがGit上で同じbranchを操作した証拠ではない。
File Provider配下へ大量のcheckout fileを作り、PC全体の同じCloudDocs queueへ負荷を加えた証拠である。

### 一objectの読取

465 byteのdataless Git objectへ `shasum` を実行した。
command runnerは30秒のyieldで実行session IDを返し、同じsessionをpollするとexit code 0で完了した。
完了後、対象fileの `dataless` flagは消えていた。

JavaScript cellの `Script completed` と、子processの完了は別である。
OpenAI Docsの公開ページでは内部のunified exec session ID契約を確認できなかったため、この部分は現在のtool contractと実測を根拠にする。

## REDとGREEN

最初の利用者向けtestはmaterialization preflightが存在しないため、次の期待どおり失敗した。

```text
worktree_guard_test=FAIL reason=missing_materialization_preflight
```

実装後、同じtestはlocal fixtureのPASS、synthetic datalessのBLOCKED、二つの同時作成の直列化、作成後のbranch、HEAD、clean status、lock消去を確認した。
さらにsynthetic File Provider属性を持つ作成先の拒否と、`.codex/worktree` をpreflightから除外しないことも確認した。

実PCでDocuments配下の未作成pathを指定した確認も、branchとpathを作る前に次の結果で停止した。

```text
worktree_creation=BLOCKED reason=destination_file_provider_managed
exit_code=75
```

```text
worktree_guard_test=PASS
```

Finder cleanup ruleをtestへ先に追加した時点では、AGENTS.mdに開始前の棚卸し規則がないため、次の期待どおりのREDになった。

```text
worktree_guard_test=FAIL reason=missing_finder_baseline_rule
```

AGENTS.mdには、開始前のwindow ID、target、bounds、情報・詳細panelの記録、終了時の新規UIのclose、再利用した既存windowの復元、終了後の再棚卸しを追加した。
Finderを終了したり、無関係な既存windowを一律に閉じたりしない。

## 残る確認

- FinderでTSUNORU folderへ「ダウンロードを保持」を適用したが、最終preflightは `dataless_git_objects=141` と `dataless_worktree_files=45` で停止中である。
- repository全体がローカルへ実体化するまで、commitを含むGit変更操作は行わない。
- scriptを通さない直接の `git worktree add` は機械的に遮断できない。AGENTS.mdを全sessionの入口とする。
- repository本体をFile Provider外へ移すには、他sessionのlinked worktreeを止め、path影響を確認する必要があるため未実施である。
