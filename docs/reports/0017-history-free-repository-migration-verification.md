# Story 0019 検証記録

Date: 2026-09-04

## 結論

TSUNORUの正本を、File Provider管理外の`$HOME/Developer/active/tsunoru`へ移した。
新しい正本はpublic GitHub repositoryからのfresh cloneであり、旧iCloud repositoryの`.git`、117commit、`target/`を引き継いでいない。

public mainは履歴なしroot commit`351bddffddd873d9b95ef55d7a7cad17b86fe8b8`から始まる。
未完了のカレンダー修正は、mainへ混ぜず`fix/calendar-layout-verification`の`f459e4786598d60fc33d749c3b370acc2a7de6a3`として保持した。

旧iCloud repositoryと既存worktreeは削除していない。
旧配置へ戻すrollback経路は、利用者が別途削除を承認するまで残る。

## 公開履歴とsource境界

移行元の検証済み公開treeと、独立repositoryでroot commit前に作ったtree objectは、どちらも次の値だった。

```text
01bc701bccf78a2714e2f7b89ac9d6ade47e202a
```

独立repositoryの`git rev-list --all --count`は、mainとカレンダーfeatureを合わせて2commitだった。
旧mainのcommit IDは`git cat-file -e`で存在しないことを確認した。
authorとcommitterのemailは`users.noreply.github.com`だけだった。

root commit前に次を確認した。

- `git diff --cached --name-only`で127 tracked fileを確認した。
- `git diff --cached --check`はexit code 0だった。
- GitHub token、OpenAI形式token、AWS access key、private key header、個人Gmail、`/Users/<name>/`のpatternは0件だった。
- `.env`、key file、credential file、SQLite、`target/`、`var/`、`.mydocs/`はtracked fileに含まれなかった。
- `scripts/test-public-snapshot-boundary.zsh`はStory IDとADR IDの一意性、worktree ignore、local state非追跡をPASSした。

新しい正本の最終`git fsck --full`では、commit前に個人absolute pathを除いた際の中間reportが、到達不能blobとして一件だけ見つかった。
`git fsck --unreachable --no-reflogs`で対象が一件だけであることと内容を確認し、`git prune --expire=now`で削除した。
prune後の`git fsck --full`は出力なしでPASSした。このblobはcommitへ到達せず、remoteへpushされていない。

GitHubの`velengel/tsunoru`は`PUBLIC`で、default branchは`main`である。
`git ls-remote --heads origin main`は`351bddffddd873d9b95ef55d7a7cad17b86fe8b8`を返した。

## mainの検証

公開前のmigration worktreeと、公開後のfresh cloneの両方で次を実行した。

```text
CARGO_TARGET_DIR=$HOME/Developer/.migration/build/tsunoru-public-main cargo test --all-targets
CARGO_TARGET_DIR=$HOME/Developer/.migration/build/tsunoru-public-main cargo test --all-targets --features server
CARGO_TARGET_DIR=$HOME/Developer/.migration/build/tsunoru-public-main cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
CARGO_TARGET_DIR=$HOME/Developer/.migration/build/tsunoru-public-main dx build --web
```

すべてexit code 0だった。
fresh cloneのweb buildはclientとserverの両方を成功として終了した。

次の運用testもfresh cloneでPASSした。

```text
public_snapshot_boundary=PASS
worktree_guard_test=PASS
```

## local state

移行前にSQLiteを開いているprocessがないことを`lsof`で確認した。
WALは0 byteであり、SHMとともに一時fileとして複製しなかった。

SQLite本体はreadonlyの`PRAGMA integrity_check`で`ok`を返した。
新旧fileのSHA-256は次の値で一致した。

```text
var/tsunoru.sqlite3
c09241930cfe39b5e23d9ab310dbf211c356166ee42c66b42837538013dc8cb3

.mydocs/tsunoru-design-domain-quiz.html
8bca7444e38e03ab676b53e8a48fc0f7f18daa1ce054566ca327039b4f772508
```

fresh cloneの`git status --ignored`では、`.mydocs/`と`var/`だけがignored local stateとして現れた。

## File Providerとworktree

新しい正本には`com.apple.file-provider-domain-id`属性がなかった。
materialization preflightは、正本とカレンダーworktreeの両方で次を返した。

```text
local_git_materialization=PASS
dataless_git_objects=0
dataless_worktree_files=0
```

repository内の`.codex/worktree/calendar-layout-verification`はguardを通して作成した。
作成直後のbranchは`fix/calendar-layout-verification`、HEADは`f459e4786598d60fc33d749c3b370acc2a7de6a3`、statusはclean、Git内部lockは不在だった。

このworktreeで`cargo test --test responsive_layout`を実行し、12件がPASSした。
Story 0015で残る配信assetと実ブラウザーの検証は、完了扱いにしていない。

## Codex path

`$HOME/.codex/config.toml.pre-tsunoru-migration-20260904`へ設定backupを作った。
trusted project sectionは旧iCloud pathから`$HOME/Developer/active/tsunoru`へ置き換えた。

## 証拠境界

- Git local state: `PASS`
- public GitHub mainとfeature branch: `PASS`
- fresh cloneのtest、Clippy、format、web build: `PASS`
- fresh cloneのGit object整合性と到達不能object不在: `PASS`
- local SQLiteとquizの複製: `PASS`
- repository内worktreeの作成と関連test: `PASS`
- カレンダーfeatureの実ブラウザー操作: `UNVERIFIED`。Story 0015で継続する。
- external deployment: `NOT APPLICABLE`。この移行では変更していない。
- physical device: `NOT APPLICABLE`。
- 旧iCloud source削除: `NOT DONE`。別の明示承認を待つ。

## rollback

新しい正本の利用に問題があれば、旧iCloud repositoryと既存worktreeへ戻れる。
新GitHub repository、staging repository、新しい正本、local state copyは、原因を確認するまで削除しない。

旧sourceを削除した後は、GitHubのmainとfeature branchがsourceの復元元になる。
SQLiteとquizはGitHubに含まれないため、旧source削除の前に新配置のlocal copyを別途保全するか、消失を受け入れる判断が必要になる。
