# ADR 0004: 共有可能なMVPをDioxus FullstackとSQLiteで支える

Status: accepted

Date: 2026-09-01

## context

ADR 0003では、Rustフロントエンドの失敗原因を切り分けるため、WebAssemblyで動くDioxusのアプリケーションシェルだけを導入した。
その構成にはルーティング、サーバー関数、データベースがない。

Product Story 1では、作成したイベントを共有URLから開く。
後続のStoryでは、別のブラウザーや端末から回答し、その結果を主催者が集計して日程を決める。
originとブラウザーに閉じる `localStorage` だけでは、複数人が同じデータを共有できない。

調査時点の最新版SQLx 0.9.0はRust 1.94.0以上を要求する。
リポジトリの最低Rustバージョン1.85とローカルのstable 1.90.0は、その条件を満たしていない。
一方、`rustup check` ではstable 1.98.0へ更新できることを確認した。

## decision

- 一つのcrateでDioxus 0.7.10の `fullstack` と `router` featureを使う。
- `dioxus::launch` が提供する組み込みAxumサーバー、SSR、静的資産配信、server functionを利用する。
- イベント、候補日時、回答、日程決定の共有データはSQLiteへ保存する。
- 非同期データアクセスとmigrationには、調査時点の最新版SQLx 0.9.0を使う。
- SQLx 0.9の要件に合わせ、最低Rustバージョンを1.94へ上げる。ローカル検証には現行stable 1.98を使う。
- SQLxとサーバー専用依存はoptional dependencyとし、Cargoの `server` featureだけから有効にする。
- SQLxは `runtime-tokio`、`macros`、`migrate`、`sqlite-bundled` だけを有効にする。SQLite実装をbundleし、開発機のsystem SQLiteとの差を持ち込まない。
- `web` と `server` を分け、WASMへサーバー専用コードやデータベースdriverを含めない。
- DB本体、WAL、SHMは `var/` に置いてGitから除外し、schema migrationだけをコミットする。
- 接続時に外部キーを有効化し、イベントと候補日時など一まとまりの書き込みはトランザクションで行う。
- 作成処理は、commit後に同じイベントを再問い合わせせず、トランザクション内で保存した公開データを組み立てて返す。commit済みなのに応答だけを失敗扱いし、再試行で重複作成する境界を作らない。
- ローカル開発は既存の8080を維持したまま候補版を8081で検証し、成功後に切り替える。

Dioxus Fullstackなら、既存のRSXと型を保ったまま、ブラウザー側の型付き呼び出しとサーバー側の処理を一つのRustコードベースで学べる。
SQLiteは外部サービスの契約や秘密情報を必要とせず、単一プロセスの初期MVPで再起動後の永続化を満たせる。

参考資料：

- [Dioxus 0.7: Fullstack](https://dioxuslabs.com/learn/0.7/essentials/fullstack/)
- [Dioxus 0.7: Fullstack Project Setup](https://dioxuslabs.com/learn/0.7/essentials/fullstack/project_setup/)
- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/)
- [Dioxus 0.7: Router](https://dioxuslabs.com/learn/0.7/essentials/router/routes/)
- [SQLx 0.9.0 crate metadata](https://crates.io/crates/sqlx/0.9.0)
- [SQLite: Write-Ahead Logging](https://www.sqlite.org/wal.html)
- [WHATWG: Web Storage](https://html.spec.whatwg.org/multipage/webstorage.html)

## rejected options

### Dioxus WebとlocalStorageだけを使う

構成は小さいが、保存内容は同じoriginの同じブラウザーに閉じる。
共有URLを別端末へ渡し、複数人の回答を一つに集める要件を満たさないため却下する。

### サーバーのメモリーだけへ保存する

別ブラウザーから同じプロセスへアクセスできる。
しかし、開発サーバーの再起動や再ビルドで全イベントが失われるため却下する。

### SQLx 0.8.6を使い、最低Rustバージョン1.85を維持する

現在のローカルtoolchainで直ちにビルドできる。
しかし、最新版を使えるRust stableが存在するのに、古いMSRVを守るためだけに一世代前のSQLxを選ぶと、近い将来に依存関係とtoolchainの両方を更新する作業が生じる。
フロントエンドの変化へ追従したいという本プロジェクトの技術方針とも合わないため却下する。

### rusqliteを使う

Dioxus公式tutorialで扱われ、SQLiteを小さく導入できる。
一方、Fullstackサーバーのasync runtimeから同期DB処理を分離する設計が必要になる。
後続Storyで回答と集計を増やすため、async pool、migration、分離DBを使ったテストを一つのlibraryで扱えるSQLxを選ぶ。

### フロントエンドとREST APIを別crateへ分ける

配備と責務を独立させやすい。
初期MVPでは型の重複、CORS、二つの開発プロセスが増え、学習対象と保守箇所が広がるため採用しない。

### 外部のmanaged databaseを使う

複数インスタンスや公開環境へ移行しやすい。
しかし、初期MVPのローカル開発に外部契約、network、credential管理を持ち込むため採用しない。

## consequences

- 共有URLを別ブラウザーで開き、同じイベントへ回答を集約する土台ができる。
- UI、通信型、server function、保存処理をRustで追跡でき、Fullstackの境界を学べる。
- 外部DBのcredentialを持たずに、再起動後もデータを残せる。
- Rust 1.85の環境ではビルドできなくなり、開発者はRust 1.94以上へ更新する必要がある。
- DioxusとSQLxの二種類のCargo featureを正しく分離する必要があり、全feature検査とWASM buildを別々に確認する必要がある。
- bundled SQLiteをコンパイルするため、system libraryを使う構成より初回build時間と成果物が増える。
- SQLite WALでも同時に書き込めるwriterは一つであり、高負荷や水平分散へ移る場合はDBの再選定が必要になる。
- DBファイルはローカル成果物なのでGitでは共有されない。schemaとfixtureはmigrationとtestで再現する必要がある。
- Dioxusのfeatureやmigrationを変えた場合、hot reloadだけでは反映されず、開発サーバーの再起動が必要になる。
- 作成応答は保存時点の公開データを返すため、DB triggerなどで値を変換する設計を後から導入する場合は、同じトランザクション内で明示的に再取得する必要がある。
