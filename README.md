# TSUNORU

TSUNORUは、友人や仲間との集まりについて、参加者から都合をつのり、主催者が開催日を決めるための日程調整アプリケーションである。
クライアントとサーバーはRustとDioxus Fullstackで実装する。
画面はブラウザー上のWebAssemblyで動き、共有データはローカルのSQLiteへ保存する。

現在は、ログインせずイベント名、任意のひとこと、候補日時を入力し、回答用の共有URLを作れる。
候補日時は、初期値 `19:00` の時刻を必要に応じて文字入力で変え、常に見える月間カレンダーの日を押して追加・解除できる。日付の直接入力もfallbackとして残している。
共有URLを開いた回答者は、名前と全候補の○、△、×を送る。保存直後には、その時点のみんなの回答一覧を同じ画面で確認し、任意のひとことを一件だけ添えられる。
主催者は作成したブラウザー、または手元に保存した復旧キーから専用画面を開き、候補ごとの件数と控えめな判断補助、最大3件のひとことpreviewを確認できる。
サマリーだけでは判断しにくいときは、必要な場合だけ回答者と候補日時の集計表を読み込み、すべての○、△、×を確認できる。
主催者は候補日時を一つ明示的に選んで確定できる。同じ候補への再試行は安全に同じ結果へ戻り、別候補で確定済みの日程を黙って上書きしない。
決定後は同じ共有URLで結果を確認でき、新しい回答は受け付けない。回答者と主催者は、決定した一件をiCalendarとして持ち帰るか、publicなイベントURLをsystem shareまたはcopyで共有できる。

履歴を使いたい場合だけ、画面上部の「履歴」からaccountを作成してloginできる。
login中に新しく主催・回答したイベントは、主催と参加に分かれて履歴へ並ぶ。
login前の匿名利用は自動で取り込まず、account sessionも主催者capabilityの代わりにはならないため、loginなしの作成・回答は従来どおり利用できる。
履歴の「当時の記録を見る」を開くと、新しい活動記録を入力せず、決定した開催日時、回答者名、候補ごとの回答、任意のひとことを振り返れる。
主催したイベントでは届いた回答全体、回答したイベントではそのaccountに直接結び付いた自分の回答だけを表示する。
この履歴詳細はread-onlyであり、account sessionだけで主催者用の操作権限を復元しない。

login中に主催したイベントの履歴詳細からは、「同じ活動の次回をつのる」を明示的に選べる。
末尾名が厳密な `名前 #N` なら次の番号を編集可能な候補として示し、名前を変えても同じ活動として主催履歴へまとまる。
通常の新規作成、似た名前、回答しただけのイベントを自動でseriesへ分類せず、session失効時も単発の匿名イベントとして黙って保存しない。

## 必要環境

- Rust 1.94以上。
- Dioxus CLI 0.7.10。

このリポジトリでは、Rust 1.98.0とDioxus CLI 0.7.10で動作を確認している。
Rustの最低バージョンは `Cargo.toml` の `rust-version`、Dioxusの版は `Cargo.lock` を正とする。

Rustを未導入の場合は、[rustup](https://rustup.rs/)の手順に従ってstable toolchainを導入する。

Dioxus CLIは、アプリケーションと同じ0.7.10を指定して導入する。
初回はCLIとその依存関係をローカルでコンパイルするため、数分かかる場合がある。

```bash
cargo install dioxus-cli --version 0.7.10 --locked
```

導入した版を確認する。

```bash
rustc --version
cargo --version
dx --version
```

## 開発サーバーの起動

リポジトリのルートで、DioxusのFullstack開発サーバーを起動する。

```bash
dx serve --web
```

Dioxus CLIはクライアントとサーバーを並列にビルドする。
初回はSQLxとSQLiteを含む依存関係をコンパイルするため、時間がかかる場合がある。

CLIが表示したURLをブラウザーで開く。
既定ではブラウザーが自動で開き、ポート8080を使用する。
ポートが使用中の場合は、ターミナルに表示されたURLを優先する。

停止するときは、開発サーバーを実行しているターミナルで `Ctrl+C` を押す。
ブラウザーを自動で開きたくない場合は、次のように起動する。

```bash
dx serve --web --open false
```

検証済み版と候補版を同時に保つ場合は、別々のターミナルでportを明示する。

```bash
dx serve --web --addr 127.0.0.1 --port 8081 --open false
dx serve --web --addr 127.0.0.1 --port 8082 --open false
```

local accountのsession cookie名はportごとに分かれるため、同じbrowserで8081と8082を開いてもsessionを上書きしない。
internetへ公開する構成では、HTTPSの完全なoriginを `TSUNORU_PUBLIC_ORIGIN` に設定する。未設定時のaccount機能はloopback HTTPだけを許す。
この環境変数はTLSを提供しない。公開時はTLSを終端し、HTTPをHTTPSへredirectしてHSTSを返すingressの背後へ置き、DioxusのHTTP listenerをinternetへ直接公開しないことも必須である。

開発中のイベントは `var/tsunoru.sqlite3` に保存される。
`var/` はGit対象外であり、開発サーバーを再起動しても内容は残る。

共有URLは、そのURLを知る人がログインなしで閲覧できる。
主催者専用操作に使うcapabilityは共有URLに含めず、通常は作成したブラウザーの `localStorage` に保存する。
ブラウザー設定などで保存できない場合は、作成成功画面に主催者用の復旧キーを一度だけ表示する。画面を閉じる前に安全な場所へ保存する。
作成成功画面の「回答サマリーを見る（主催者用）」から専用画面へ進む。別のブラウザーで開く場合は、専用画面へ復旧キーを入力する。
回答後のひとことに使う回答capabilityは、回答直後の画面が開いている間だけメモリーに保持し、URL、SQLite、`localStorage`には保存しない。
イベント名やひとことへ、公開したくない情報を入力しないこと。

accountはlogin IDとpasswordだけで作成する。
現在はpassword再設定を用意していないため、失うとそのaccountの履歴を復旧できない。
passwordの平文と生のsession tokenはSQLiteへ保存せず、sessionはHttpOnly cookieとして扱う。
logoutしても、匿名で作成したイベントを管理するための主催者capabilityはbrowserから削除しない。

## 検証コマンド

利用者に見えるHTMLをテストする。

```bash
cargo test --all-targets
```

SQLiteのmigration、transaction、公開eventの読み込みを含むserver側をテストする。

```bash
cargo test --all-targets --features server
```

Web向けdefault featureを外したserver構成もテストする。

```bash
cargo test --all-targets --no-default-features --features server
```

Rustコードを静的検査する。

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Rustコードの整形を検査する。

```bash
cargo fmt --check
```

Fullstackのクライアントとサーバー成果物をビルドする。

```bash
dx build --web
```

## ディレクトリ構成

```text
assets/       ブラウザーへ配信するスタイル
docs/         Story、ADR、検証記録、用語集
migrations/   SQLite schemaの履歴
scripts/      Gitのローカル実体化、worktree作成、運用test
src/domain.rs クライアントとサーバーで共有する入力、公開event、検証
src/lib.rs    Dioxusアプリケーションと型付きroute
src/calendar.rs 一件の決定済みイベントを安全なiCalendarへ変換
src/auth.rs  password hash、session token、cookie、same-origin境界
src/server.rs Dioxus server functionとraw iCalendar response
src/storage.rs server featureだけで使うSQLite repository
src/ui.rs     作成、回答、主催者用集計、日程決定、持ち帰り、account履歴・継続画面
src/main.rs   Fullstackアプリケーションの入口
tests/        HTML、入力検証、保存境界の受け入れテスト
var/          ローカルSQLiteデータ。Git対象外
```

## 開発の進め方

macOSでは最初のrepository全体を読むGit commandより先に、Git objectと作業fileがローカルへ実体化されているか確認する。

```bash
zsh scripts/verify-local-git-materialization.zsh
```

`dataless_files_present`ならGit操作を重ねず、Finderでrepository folderを右clickして「ダウンロードを保持」を選ぶ。
保持を指定済みでもpreflightがPASSするまでは完了ではない。

worktreeはFile Provider domain外へ一件ずつ作る。
正本が`$HOME/Developer/active/tsunoru`にある場合は、ignoredなrepository内directoryを使う。

```bash
zsh scripts/create-feature-worktree.zsh \
  feature/example \
  "$PWD/.codex/worktree/example" \
  main
```

作成先または既存ancestorにFile Provider domain属性があれば、scriptはbranchを作る前にexit code 75で停止する。
command runnerが実行session IDを返した場合は、同じsessionを完了まで待つ。

Finderを開く調査では、操作前に既存windowと情報・詳細panelを棚卸しする。
終了時は今回増やしたwindow、panel、sheetだけを閉じ、再利用した既存windowは元のtargetとboundsへ戻したうえで、残存UIを再確認する。

実装より先に `docs/story/` のStoryと、必要な `docs/ADR/` の意思決定を更新する。
受け入れテストを先に失敗させ、最小の実装で通し、回帰検証を終えてからコミットする。

ローカルコードレビューでは [レビュー判断履歴](docs/review-judge-logs.md) を読み、現在の差分と照合する。
レビュー対応はPRごとに最大2往復で一旦止める。指摘の有無だけで修正を決めず、元の目的への影響と対応コストを判断する。上限では残件とマージ可能性を報告し、明示的な指示なしに3往復目へ進まない（[ADR 0043](docs/ADR/0043-stop-review-follow-up-after-two-rounds.md)）。
受信した全指摘と関連する箇所をまとめて確認し、修正と検証を収束させてから一度の再レビューへ進む。
対応したPRコメントには、push済みの修正コミットと検証結果を添えて返信する。

コミットは一つの作業区切りに絞り、subjectを `<prefix>: <summary>`、bodyを `why` と `what` で構成する。
token、API key、password、private key、実値入りの環境ファイルはコミットしない。

## Calendar browser regression

After `dx build --web`, run the repository-owned browser verifier with an installed Playwright module.
Playwright 1.62.0 and its Chromium 151 build were used for the recorded verification.
The fixture database uses the built-in `node:sqlite` module; Node 25.9.0 was used for these checks.
The verifier seeds a private database marker and confirms it through a read-only API before permitting test writes.
Each mutation uses the same TCP connection as its identity check, with automatic reconnection disabled; a replaced listener cannot receive that mutation.
The served-asset checker uses Python 3 to inspect responses in memory, without temporary files.

```sh
PLAYWRIGHT_MODULE=/path/to/playwright/index.mjs node scripts/verify-calendar-browser.mjs
PLAYWRIGHT_MODULE=/path/to/playwright/index.mjs node scripts/verify-calendar-browser.mjs --stale-css
PLAYWRIGHT_MODULE=/path/to/playwright/index.mjs node scripts/test-calendar-browser-shutdown.mjs
PATH="$PWD/scripts/fixtures/portable-process-tools:$PATH" PLAYWRIGHT_MODULE=/path/to/playwright/index.mjs node scripts/test-calendar-browser-shutdown.mjs
python3 scripts/test-runtime-shutdown.py
python3 scripts/test-runtime-source-snapshot.py
python3 scripts/test-runtime-server-identity.py
PLAYWRIGHT_MODULE=/path/to/playwright/index.mjs python3 scripts/test-browser-server-identity.py
PLAYWRIGHT_MODULE=/path/to/playwright/index.mjs python3 scripts/test-harness-termination.py
python3 scripts/test-calendar-assets.py
python3 scripts/test-harness-process-groups.py
node scripts/test-verification-connection.mjs
python3 scripts/test-verification-connection.py
```

The runner starts this worktree's built server on its own loopback port with a disposable database and fresh browser profile.
It checks the linked stylesheet against the bundled file, 320px and 1440px geometry, keyboard controls, and event creation through the post-answer matrix.
The negative fixture checks that an HTTP 200 response containing old CSS is rejected.
The HTTP verifier reads source DB/WAL/SHM files as bytes and performs SQL inspection only on a disposable snapshot.
The source must be quiescent: changing file sets fail verification instead of being treated as a valid online backup.
Screenshots and measurements are saved under ignored `var/browser-evidence/`; test data and owned processes are removed on exit.
Each concurrently running build must use a different worktree or target directory, not just a different port.

This is a desktop Chromium check. Physical iPhone operation and screen-reader speech require separate verification.
