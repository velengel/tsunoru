# 既存Cloudflare構成とTSUNORUの差分

2026-09-05。ユーザーの既存運用を優先する条件を受けた追加調査。
結論はCloudflare優先へ変更する。
Rustの差分は存在するが、既存運用を覆して別ホストを先に選ぶだけの実証はない。

## 実際に確認した既存構成

参照元は `/Users/velengel/Developer/active` 以下のローカルcheckout。
外部の稼働状態、課金プラン、アカウントの権限は今回確認していない。
他repoは読み取りだけで、DB、secret、環境ファイルは読んでいない。

| repo | 根拠ファイル | 構成と再利用できる経験 |
| --- | --- | --- |
| koji-todo | `wrangler.jsonc:4`、`:7`、`:13`、`src/adapters/d1/task-repository.ts:133` | TypeScript Worker、Static Assets、D1、R2、batch。静的UIとAPIを同一originで公開する構成が近い |
| koji-todo | `src/auth.ts:178`、`:204` | Apple認証とsession。TSUNORUの匿名参加/任意accountとは製品上の認証要件が違う |
| voice-workbench | `cloud/worker/wrangler.jsonc`、`cloud/worker/src/index.ts:337`、`:751` | TypeScript Worker、D1、R2、Containers、Durable Objects。native処理をcontainerへ分ける構成も既にある |

Wrangler、環境分離、D1 migration、version確認、運用手順の考え方を参考にできる。
既存DB/secretやApple認証をそのままTSUNORUへ流用することとは分ける。
R2、Queues、Containersなどを実績があるという理由だけで追加しない。

## Rust固有の差分と判定

| 差分 | 根拠 | 変更の見込みと判定 |
| --- | --- | --- |
| native serverからWasm実行へ | TSUNORU `src/main.rs:5`、Cargo.toml | entrypoint、依存feature、非同期実行の適合確認が必要。Rust全廃は不要 |
| SQLxのファイルSQLiteからD1へ | `src/storage.rs:26`、`:922` | D1 bindingによる保存層へ変更。ファイルpathを書き換えるだけでは済まない |
| 複数SQLの一体処理 | `src/storage.rs:1323`、`:1955`ほか | batchや条件付きSQLへ再設計。read後の分岐、更新0件、競合時rollbackを既存テストで守る |
| password計算 | `src/auth.rs:25`、`:52`、`:71` | spawn_blockingを前提にしない実行方式と、Argon2のCPU/メモリ実測が必要。hash強度を下げて合わせない |
| Dioxus Fullstack接続 | server functions、SSR、hydration | Dioxus公式はWorkersをWasm配置先に挙げるが、TSUNORU 0.7.10の依存一式の稼働証明ではない |

Cloudflare公式[workers-rs](https://github.com/cloudflare/workers-rs/blob/main/README.md)はRust→Wasm、Axum接続例、D1 bindingを提供する。
[Dioxus Fullstack](https://dioxuslabs.com/learn/0.7/essentials/fullstack/)と[配布説明](https://dioxuslabs.com/learn/0.7/tutorial/deploy/)もCloudflare Workersを配置先に挙げている。
したがって「Rust/AxumだからCloudflareでは難しい」という一般化は強すぎる。

[D1 batch](https://developers.cloudflare.com/d1/worker-api/d1-database/)はSQL失敗時に全体rollbackする。
ただし条件付きUPDATEが0件だっただけではSQLエラーにならないため、現行の競合判定を単純なbatch化で守れたとは言えない。
[Workersの制限](https://developers.cloudflare.com/workers/platform/limits/)も踏まえ、Argon2を無料枠で動かせると仮定せず、既存契約条件と実測を照合する。

## 次の小実験

第一候補は `Dioxus UI + Rust Worker + D1 + Static Assets`。
UIと業務ルールの再利用を先に試し、全面TypeScript化や別認証方式を前提にしない。

1. Dioxus 0.7.10の最小server functionをRust Workerから応答させ、UIから同一originで呼び出す。SSR/hydration、CookieとOrigin拒否を確認する。
2. 使い捨てD1でイベント＋候補の作成と匿名回答を動かす。途中失敗、再送、競合で部分保存と重複が生じないことを確認する。
3. 続きイベントのexpected tail競合、session失効など、最も難しい認可/transaction経路を一つ通す。単純CRUD成功だけで移植可能としない。
4. 現行パラメータのArgon2を実測し、時間、メモリ、並列実行、資源制限、費用を記録する。

最初の1作業日を目安に、通った範囲、障害、残りの保存層移植量を整理して全面移植の可否を判断する。
ローカル成功とCloudflare上の成功を分ける。
既存アプリのDBを検証用に使わない。

Wasm経路に具体的な障害があれば、voice-workbench型のWorker＋Rust Container＋外部永続DBを比較する。
Container内SQLiteの非永続性は残るので、container化だけで解決したとしない。
それでも総工数や運用費の不利が大きい場合に、Render等へ戻る。
今回、コンパイル実験やデプロイは行っておらず、Cloudflare互換性はUNVERIFIED。

## 計画への反映

[ADR 0045](../ADR/0045-prioritize-existing-cloudflare-operations.md)がADR 0044の評価順を置き換える。
[元計画](0021-publication-plan.md)の認可、スマホ、公開前の安全条件は維持する。
native disk固有の配布/backup手順はCloudflare案の採用手順とは扱わず、D1採用時はTime Travel、export/import、復元後の失効/削除再適用へ具体化する。
元の敵対的検証1往復は完了済み。今回のユーザーによる前提修正で自動レビューを再開しない。

Surprise & Discovery: voice-workbenchにはContainersの構成も存在し、Cloudflare運用に寄せる選択肢はWorkers/Wasmだけではなかった。
