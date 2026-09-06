# レビュー判断履歴

## PR #13 のローカルレビュー（2026-09-06）

R040–R043 と R018/R020/R023 を参照し、限定試用の目的、Cookie の導入、同一 origin の CSR 配信から再評価した。
Worker の独立した self-review では、主催者 hash、Cookie と Origin、日時、同じ D1 batch 内の認可に追加の要対応指摘はなかった。
実装中とブラウザー検証で見つけた以下の問題はまとめて修正した。
外部の review comment に対する返信ではなく、[PR #13](https://github.com/velengel/tsunoru/pull/13) のローカル判断である。

| ID | 判断 | 理由と証拠 |
| --- | --- | --- |
| R045 | 修正 | 自前の eval を除いても `document::Title` が固定版 Dioxus 内部で eval を呼び、CSP と衝突する。静的 HTML のタイトルを使用し、ブラウザー操作後の error/warn 0件を確認 |
| R046 | 修正 | release CSR build の wasm-opt が DWARF で失敗しても CLI が0で終了した。`--debug-symbols=false` を固定し、最適化ログと実際の Wasm 操作を確認 |
| R047 | 修正 | Dioxus の出力に旧 hashed assets が残った。公開用の生成ディレクトリを再作成してからビルドし、最終6ファイルと SHA-256 を記録 |
| R048 | 修正 | 320px viewport の有効幅305pxに対して html/body の最小幅320pxが残り、横にはみ出した。限定版で両方を解除し、作成と集計の client/scroll 幅305/305pxを確認 |
| R049 | 修正 | 不正な zone などの400でも送信内容を固定し続けると入力を訂正できない。DB操作前の `invalid_request` だけで pending を解除し、元の項目をフォームへ戻す。通信失敗や401/409/500では保持。ブラウザーで訂正後の作成まで確認 |

実装と検証: [48a64eb](https://github.com/velengel/tsunoru/commit/48a64eb)、[report 0028](reports/0028-staging-browser-app.md)。
CSS や Rust の文字列をなぞる2試験は採用せず、保存と復元、表示される結果の9試験、実ブラウザーの測定を根拠にした。
account、個別失効、回答編集、旧 DB migration の追加は今回採らず、[#12](https://github.com/velengel/tsunoru/issues/12) で要否を判断する。

hosted Codex review は2往復を完了し、R050/R051 を修正・返信・解決した。最後にレビューされた head は `6df16f7` で、その後の R051 修正と判断記録は再レビューしていない。上限に従い、3回目は依頼しない。
remote D1 作成の承認待ちはコードレビューの承認と分ける。

### R050: ビルド失敗時の生成物（初回 hosted review）

2026-09-06。[review summary](https://github.com/velengel/tsunoru/pull/13#issuecomment-5555942089) は `109d276` を対象に完了し、[指摘](https://github.com/velengel/tsunoru/pull/13#discussion_r3942568289) は1件だった。

**修正**。途中の Worker build が失敗すると、新しい assets と不完全な Worker が公開候補のディレクトリに混ざる。限定試用を安全に配置する目的に直接関係し、合成 CLI でも再現したため、必要な対応と判断した。

[b99ef89](https://github.com/velengel/tsunoru/commit/b99ef89723b1364354c4fb1521573ed774672781) で Worker と assets を一時 bundle にまとめ、両方の成功後に切り替える。失敗・SIGINT・SIGTERM・成功の4ケースで出力と所有子プロセスを確認し、実際の dry-run と Worker HTTP 試験も通した。[report 0028](reports/0028-staging-browser-app.md) に証拠を記録した。

検証・push 後に [コミット付きの対応返信](https://github.com/velengel/tsunoru/pull/13#discussion_r3942587174) を送り、thread の `isResolved: true` を確認した。初回の1往復はここで完了。最終確認は残り1回以内とし、結果を転記するだけの未レビュー commit は追加しない。

### R051: 失敗したコマンドの補助プロセス（2往復目）

2026-09-06。`6df16f7` に対する [2往復目の review](https://github.com/velengel/tsunoru/pull/13#pullrequestreview-5123672225) は完了し、新しい [指摘](https://github.com/velengel/tsunoru/pull/13#discussion_r3942598160) は1件だった。

**修正**。実 CLI での発生は未観測だが、コンパイラーが失敗して補助プロセスだけが残る合成ケースで再現した。公開画面の不具合とは区別する。所有プロセスを残さない既存要件に合い、対象を終了処理と再現試験に絞れるため、今回は対応する。

[93b3633](https://github.com/velengel/tsunoru/commit/93b36337200bfca07082836ae80d0e30444850aa) で失敗後も group ID を保持し、TERM、期限付き待機、必要時の KILL を行ってから pipe の終了を待つ。TERM を無視する補助プロセスを含め、失敗・SIGINT・SIGTERM・成功の4ケースと実際の dry-run が通った。[report 0028](reports/0028-staging-browser-app.md) に再現と検証を記録した。

検証・push 後に [コミット付きの対応返信](https://github.com/velengel/tsunoru/pull/13#discussion_r3942606217) を送り、thread の `isResolved: true` を確認した。受領した指摘はすべて判断済みで、2往復を終える。この修正と履歴の commit は hosted review 未実施であり、承認済みとは扱わない。

## PR #10: #9 の判断を実装から再評価（2026-09-06）

R031、R034–R039 を読んでから `492506e` のコードと native の回答契約を照合した。これは #9 に新しい hosted review を要求するものではない。#10 の初回 hosted review は `7f2e6e0` で完了し、指摘0件だった。最大2往復の上限を引き継ぐ。

| ID | 判断 | 理由と証拠 |
| --- | --- | --- |
| R040（R034/R036 再評価） | 修正 | #9 は候補単位の upsert とイベント共通鍵で、全候補回答・回答者間の所有権を保証しなかった。回答単位の hash と全件 batch に置換。`cloud/rust-worker/src/api.rs`、HTTP の欠落・重複・同名・同時再送・競合試験で確認 |
| R041（R031 再評価） | 修正 / 一部保留 | 入口の Bearer と完全一致 Origin を DB 前に検証する。Cookie を使わない API に account/CSRF token を同時実装する必要はない。一般公開の制限・個別失効は [#12](https://github.com/velengel/tsunoru/issues/12) で要否判断 |
| R042（R038 再評価） | 保留 / 誤適用防止 | 実データを持つ staging DB は未作成で、新規 baseline に限定する。`IF NOT EXISTS` を外し既存表には失敗させる。migration/restore は [#12](https://github.com/velengel/tsunoru/issues/12) へ引き継ぐ |
| R043（検証の再現性） | 修正 | 別 repo の絶対 import を廃止し lockfile 固定、Miniflare を終了時回収。SIGINT/SIGTERM で所有 workerd と一時データ消滅を実測 |
| R044（PR本文） | 修正 | #9 でユーザーが要求した本文同期が運用へ未記録だった。ADR 0054 と AGENTS に追加し、最終本文を GitHub から読み戻す |

元の指摘: [候補集合](https://github.com/velengel/tsunoru/pull/9#discussion_r3941003984)、[回答の識別](https://github.com/velengel/tsunoru/pull/9#discussion_r3941003989)、[migration](https://github.com/velengel/tsunoru/pull/9#discussion_r3941003994)。R040–R044 の実装と検証は [7f2e6e0](https://github.com/velengel/tsunoru/commit/7f2e6e02d8ca323deda7a9d90766717d151620b8)、詳細は [report 0027](reports/0027-staging-authorization.md)。

初回の [Codex summary](https://github.com/velengel/tsunoru/pull/10#issuecomment-5553168882) は 2026-09-06 01:29 JST に Completed。対象 head が実装 commit と一致し、reviews・inline threads は0件、Codex の thumbs-up を確認した。GitHub の承認 review が付いたという意味ではない。対応対象の外部コメントがないため、返信・スレッド解決も不要だった。

本文の読み戻し、実装 commit のリンク、Story 完了項目を整合させる文書変更でこの作業を閉じる。残り1回以内で最終 head をレビューし、その結果は PR に残す。結果の転記だけを目的とする追加 commit は作らない。

## R034-R039: Codex review for PR #9 (2026-09-06)

- R034（候補日モデル）: **修正**。候補日テーブルと候補日単位の回答キーを追加した。コメント: https://github.com/velengel/tsunoru/pull/9#discussion_r3941003984
- R035（availability enum）: **修正**。Worker検証とD1 CHECK制約を追加した。コメント: https://github.com/velengel/tsunoru/pull/9#discussion_r3941003988
- R036（表示名の主キー）: **修正**。表示名を識別子にせず、response_id と候補日を冪等キーにした。コメント: https://github.com/velengel/tsunoru/pull/9#discussion_r3941003989
- R037（capability平文）: **修正**。SHA-256を保存・照合に使う。コメント: https://github.com/velengel/tsunoru/pull/9#discussion_r3941003992
- R038（既存schema migration）: **保留**。専用staging新規D1が前提で、既存DB移行は別Issueに切り出す。コメント: https://github.com/velengel/tsunoru/pull/9#discussion_r3941003994
- R039（Story同期）: **修正**。実装・ADR完了項目のチェックを更新した。コメント: https://github.com/velengel/tsunoru/pull/9#discussion_r3941003995

## R031-R033: staging event API (2026-09-06)

- R031（認証）: **要対応（次段階）**。capability照合は追加したが、Cookie session、失効、Origin/CSRFは未実装。本番公開せず、Story 0028 の次作業へ切り出す。
- R032（入力・D1）: **修正済み**。イベント作成と匿名回答の必須値をWorker側で検証し、分離schemaに外部キーと複合主キーを追加した。検証は `cloud/rust-worker/verify-local.mjs`。
- R033（スコープ）: **変更不要**。本番resourceやUI接続を同時に扱わず、staging APIの縦切りに限定した。

## PR #8 小実験のローカルレビュー

2026-09-05。[PR #8](https://github.com/velengel/tsunoru/pull/8)。既存R018/R019/R023とコードを照合し、以下を一括判断した。
GitHubレビューコメントではなく、実験実装のローカル判断である。hosted指摘は確認時0件。

| ID | 判断 | 理由と証拠 |
| --- | --- | --- |
| R025 | 修正 | domain共有時にserver featureを外すと未知timezoneを受け入れる。featureを有効化し、workerdで未知zone拒否を検証した |
| R026 | 修正 | raw Wasm/D1試験をFullstack認証の成功と誤読しないよう、JS入口、合成schema/session、32-bit fingerprint、未検証のCookie/PHC/CPU課金を明記した |
| R027 | 今回採用しない | Dioxus依存のforkや全面TypeScript化は小実験の目的を超える。serverの実ビルド失敗とbrowser PASSを残して別判断にする |

実行結果、修正ファイル、再現方法は[結果](reports/0023-cloudflare-runtime-spike.md)と[HTTP検証](../experiments/cloudflare/verify.mjs)を参照。
修正と証拠のcommit: [8ae4fd8](https://github.com/velengel/tsunoru/commit/8ae4fd8)。検証後push済み。GitHub上の対応対象コメントはないため返信は行っていない。

## PR #7 公開計画の敵対的検証1往復

2026-09-05。[PR #7](https://github.com/velengel/tsunoru/pull/7)の初版[ dcae200 ](https://github.com/velengel/tsunoru/commit/dcae200)を対象に、原目的から過剰実装と公開事故の両方を検討した。
R017および既存の権限保護ルールを参照したうえで、ローカル検証1回と計画修正1回で停止する。
以下はローカルでまとめた指摘であり、GitHub上のレビューコメントではない。
修正版は[公開計画](reports/0021-publication-plan.md#敵対的検証後の補足)。計画修正commitは[3d48b70](https://github.com/velengel/tsunoru/commit/3d48b70)。
文書内の相対ファイル参照、差分整形、staged secret scanがPASSでpush済み。
確認時のPRレビュー/コメント/CIチェックは0件であり、hosted reviewの承認は主張しない。

| ID | 指摘と根拠 | 判断と理由 | 修正後の検証/残件 |
| --- | --- | --- | --- |
| R018 | 高: 「入口の制限」だけでは実装完了を判定できず、偽造Originや直アクセスもある。auth.rsのOrigin検査はbot認証ではない | 計画を修正。client信頼境界、暫定上限、再起動/連打/再送試験を追加 | 文書照合済み。防御実装は公開前必須、未実施 |
| R019 | 高: 復元が失効sessionや削除済み予定を復活させ得る。account_sessionsもDBにある | 計画を修正。session全失効、削除再適用、漏えい時のアクセス停止を復元条件にする | 手順条件を確認。復元演習は未実施 |
| R020 | 高: 削除UIを保留すると保持/問い合わせまで保留になる恐れ | 計画を修正。運用者の削除手順と保持説明を公開条件に分離 | 自動化は別Issue候補、運用者/期間は公開前に確定 |
| R021 | 中: 有料ホスト推奨に費用と負荷の裏付けが不足 | 計画を修正。単価取得不可を明記し、契約前見積と小負荷試験をgateにする | 金額/性能はUNVERIFIED。契約確定ではない |
| R022 | 中: backupが静かに失敗するとRPOを守れない | 計画を修正。監視、通知、受付停止、担当確定を追加 | 実運用は未検証。監視基盤は自作しない |
| R023 | 中: 別端末から主催できないので即座に全員OAuth化すべきか | 採らない。accountとcapabilityの認可は別で、OAuthだけでは権限が移らない。端末間回復は独立Issue候補 | 初回は同じbrowserで主催、別端末は参加の条件を説明。需要で再判断 |
| R024 | 低: R017をLinux公開のついでに今すぐ直すべきか | 保留。現段階は文書のみでharnessをLinux実行しない。必要になった段階1で別Issue判断 | 既存の未修正記録を維持。公開サーバーの不具合とは扱わない |

このバッチで5件の計画修正、1件の代案不採用、1件の保留を判断した。
追加レビューを依頼せず、修正箇所と参照の整合性確認だけで終了する。

PRの指摘について、判断、理由、検証、修正へのリンクを残します。
ローカルコードレビューでは関連する履歴を読み、現在の差分と照合します。
過去の判断は新しい指摘を除外する根拠にはせず、再判断が必要なら同じ項目に日付と理由を追記します。

各項目には判断日、対象PRと指摘、対応要否、理由、状態、修正コミット、検証根拠、返信リンクを記録します。
「対応不要」や保留の場合も理由を残します。
最終コミットの外部レビュー完了状態はPR上で確認します。
履歴の状態更新だけのために、レビュー済みコミットの後へ未レビューのコミットを追加しません。

## 2026-09-05 PR #6

[PR #6](https://github.com/velengel/tsunoru/pull/6)のカレンダー検証と、Codexレビュー設定の追加が対象です。
以前の対応もこのPRで記録し直しました。
詳しい再現、測定値、制約は[検証記録](reports/0019-calendar-browser-verification.md)にあります。

### R001 ADRの独立した決定

- 指摘：[3939384154](https://github.com/velengel/tsunoru/pull/6#discussion_r3939384154)
- 判断：対応が必要。ADR 0021は今回追加した文書で、過去の採用済みADRの例外に該当しません。複数の決定を分割しました。
- 状態：修正、検証、push、スレッド解決済み。
- 修正：[79b5618](https://github.com/velengel/tsunoru/commit/79b56180912aa16be4fcd33952fe35cdcacc589d)
- 検証：ADRの必須節、decisionの一行、参照先を検査。
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939653562)（修正コミットと検証結果を通知済み）。

### R002 Node検証の終了シグナル

- 指摘：[3939426271](https://github.com/velengel/tsunoru/pull/6#discussion_r3939426271)
- 判断：対応が必要。直接SIGTERM/SIGINTを受けるとfinallyだけでは資源を回収できません。共通の終了処理を呼ぶhandlerを追加しました。
- 状態：修正、検証、push、スレッド解決済み。
- 修正：[a333020](https://github.com/velengel/tsunoru/commit/a3330205c11a6df0caad8ca7883898fb14d55bc8)
- 検証：両シグナルでサーバー、ブラウザー、一時DBの回収を検証。
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939654262)（修正コミットと検証結果を通知済み）。

### R003 worktreeとdraft PRの運用判断

- 指摘：[3939444599](https://github.com/velengel/tsunoru/pull/6#discussion_r3939444599)
- 判断：対応が必要。worktree再利用と実装前draft PR作成は別々の運用判断です。ADR 0031と0032へ分けて記録しました。
- 状態：修正、検証、push、スレッド解決済み。
- 修正：[8f0b88a](https://github.com/velengel/tsunoru/commit/8f0b88a9a5d29b22f9f56a1e46c42762dd8c203d)
- 検証：ADRの構造と既存方針との整合性を検査。
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939655230)（修正コミットと検証結果を通知済み）。

### R004 Python検証のSIGTERM

- 指摘：[3939444603](https://github.com/velengel/tsunoru/pull/6#discussion_r3939444603)
- 判断：対応が必要。Pythonの標準SIGTERMではfinallyが動かずサーバーが残ります。既存cleanupを通る終了処理に変更しました。
- 状態：修正、検証、push、スレッド解決済み。
- 修正：[8f0b88a](https://github.com/velengel/tsunoru/commit/8f0b88a9a5d29b22f9f56a1e46c42762dd8c203d)
- 検証：SIGTERM/SIGINTと通常HTTP操作を隔離fixtureで検証。
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939655341)（修正コミットと検証結果を通知済み）。

### R005 Node資源取得前のhandler登録

- 指摘：[3939464122](https://github.com/velengel/tsunoru/pull/6#discussion_r3939464122)
- 判断：対応が必要。handler登録前に資源を取得する区間があり、一時ディレクトリ残存を再現しました。登録を先に移しました。
- 状態：修正、検証、push、スレッド解決済み。
- 修正：[74d5c9d](https://github.com/velengel/tsunoru/commit/74d5c9d2b6961836c7619e504b289de28e7f4e27)
- 検証：取得途中を含む4段階で両シグナルを検証。
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939655396)（修正コミットと検証結果を通知済み）。

### R006 同期asset検査の中断不能

- 指摘：[3939494231](https://github.com/velengel/tsunoru/pull/6#discussion_r3939494231)
- 判断：対応が必要。同期子プロセスがNodeのイベント処理を止め、中断cleanupが始まりません。所有する非同期プロセス群へ変更しました。
- 状態：修正、検証、push、スレッド解決済み。
- 修正：[6ef15b7](https://github.com/velengel/tsunoru/commit/6ef15b756f4981df7f5722be499ac113a938ab14)
- 検証：停止するasset fixtureを含む5段階で両シグナルを検証。
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939655430)（修正コミットと検証結果を通知済み）。

### R007 Chromium起動待ちのcleanup上限

- 指摘：[3939552354](https://github.com/velengel/tsunoru/pull/6#discussion_r3939552354)
- 判断：対応が必要。起動promiseが未完了のままでは他の資源も回収されません。待ち時間を制限し、失敗終了後もサーバーと一時DBを回収します。
- 状態：修正、検証、push、スレッド解決済み。
- 修正：[1f014fb](https://github.com/velengel/tsunoru/commit/1f014fbcda887081c47c608712194802b85d0b85)
- 検証：未完了promiseと実Playwrightの停止プロセスを含む14ケースがPASS。
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939655500)（修正コミットと検証結果を通知済み）。

### R008 Python子プロセスの代入前中断

- 指摘：[3939580165](https://github.com/velengel/tsunoru/pull/6#discussion_r3939580165)
- 判断：対応が必要。Popenが子を生成してから管理変数へ代入する前の中断でサーバーが残りました。代入が終わるまでPythonの終了処理を保留します。
- 状態：修正、検証、push、スレッド解決済み。
- 修正：[f5018aa](https://github.com/velengel/tsunoru/commit/f5018aab1a9d0608195f854aeecf9c29cc574e62)
- 検証：代入前と起動完了後の2段階×2シグナル、通常HTTP操作がPASS。
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939655546)（修正コミットと検証結果を通知済み）。

### R009 OS依存のプロセス検索

- 指摘：[3939600856](https://github.com/velengel/tsunoru/pull/6#discussion_r3939600856)
- 判断：対応が必要。ps commのパス表記とlsofに依存していました。PIDと一時パスを明示通知し、解析エラーをpromiseへ渡します。
- 状態：修正、検証、push、スレッド解決済み。
- 修正：[4bd1676](https://github.com/velengel/tsunoru/commit/4bd16767095ba3b60ca4971db1d410a6375e46b8)
- 検証：非移植的なps検索とlsofを拒否する固定fixtureで14ケースPASS。Linux環境での実行自体は未検証。
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939655596)（修正コミットと検証結果を通知済み）。

### R010 別サーバーへの誤書き込み

- 指摘：[3939636006](https://github.com/velengel/tsunoru/pull/6#discussion_r3939636006)
- 判断：対応が必要。ポート予約を解放してから子がbindするまでに別サーバーが取得すると、HTTP 200と同一CSSだけでは自分の検証先と証明できません。
- 状態：両検証ツールの修正、検証、push、返信、スレッド解決済み。
- 修正：[9d78ade](https://github.com/velengel/tsunoru/commit/9d78ade1d2030b82d1f517bd11368aed16d94f9a)
- 検証：別のテスト専用DBに修正前はブラウザーが2件、Pythonが1件を書き込む負例を再現。修正後は両方が書き込み前に拒否し、通常操作と中断18ケースもPASS。
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939699935)

### R011 元DBのWAL補助ファイル

- 指摘：[3939636008](https://github.com/velengel/tsunoru/pull/6#discussion_r3939636008)
- 判断：対応が必要。SQLiteのmode=roでも共有メモリ用の補助ファイルが変わり得るため、主DBだけのハッシュでは元データ全体の不変を確認できません。
- 状態：使い捨てコピーだけでSQL検査する修正、検証、push、返信、スレッド解決済み。
- 修正：[9d78ade](https://github.com/velengel/tsunoru/commit/9d78ade1d2030b82d1f517bd11368aed16d94f9a)
- 検証：WALが残りSHMがない負例で、修正前の元ファイル変更を再現。修正後はWAL内だけのイベントをコピーから読み、元DBと補助ファイルの内容と有無が不変であることを確認。
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939699978)

### R012 Python一時ディレクトリの生成中断

- 指摘：[3939721392](https://github.com/velengel/tsunoru/pull/6#discussion_r3939721392)
- 判断：対応が必要。TemporaryDirectoryのfinalizer登録前にmkdtempが中断されると、まだ終了処理に登録されていないディレクトリが残ります。
- 状態：生成直後の残存を再現し、修正、検証、push、返信、スレッド解決済み。
- 修正：[5e9e630](https://github.com/velengel/tsunoru/commit/5e9e630f5eda2eaeda455767ae898135f2524cc4)
- 検証：ディレクトリ生成、子プロセス代入、起動完了の3段階×2シグナルと、通常HTTP、別DB拒否、WAL復元と元DB不変がPASS。
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939738154)
- 履歴：R008で採用した子プロセスの終了保留を、一時ディレクトリの所有権登録にも拡張しました。以前の必要対応という判断は維持します。

### R013 外側のテストドライバーの中断

- 指摘：[3939750034](https://github.com/velengel/tsunoru/pull/6#discussion_r3939750034)
- 判断：対応が必要。内側の検証ツールがシグナルを処理しても、外側のPythonドライバーはSIGTERMでfinallyを通らず、子サーバーと一時データが残りました。Nodeと他の3ドライバーにも同じ管理上の不足があります。
- 状態：5ドライバーを一括修正し、検証、push、返信、スレッド解決済み。
- 修正：[fab63dc](https://github.com/velengel/tsunoru/commit/fab63dc330aae66fc5c4f03a807e5f642c9b0a45)
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939796788)
- 検証：修正前に外側SIGTERMで子サーバー残存を再現。修正後は外側10ケース、内側Python6ケース・Node14ケース、通常HTTP、別DB拒否2種、WAL復元と元DB不変がPASS。
- 履歴：R002、R004からR009、R012の終了処理の判断を、呼び出し元にも適用しました。一時資源の登録、所有プロセスの停止、一時ディレクトリの削除順序、新しい回帰テスト自体の終了処理をまとめて確認しました。[ADR 0039](ADR/0039-scope-regression-harness-resources.md)に判断を記録します。

### R014 CSS確認の一時ディレクトリ生成

- 指摘：[3939819165](https://github.com/velengel/tsunoru/pull/6#discussion_r3939819165)
- 判断：対応が必要。実際のシェルチェッカーではmktempの生成から代入・trap登録までに中断されると、一時ディレクトリが残ります。従来の停止用fixtureでは、この実処理の隙間を検証していませんでした。
- 状態：実チェッカーで修正前の残存を再現し、一時ファイルを使わない実装へ変更、検証、push、返信、スレッド解決済み。
- 修正：[42fffe1](https://github.com/velengel/tsunoru/commit/42fffe19c214279a61a65be894c054723a050878)
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939854969)
- 検証：実チェッカーのSIGTERM／SIGINT、正常CSS、古いCSS、不正Content-Type、HTML目印の欠落がPASS。実ブラウザー320px／1440px、古いCSS検出、ブラウザー中断14ケースもPASS。
- 履歴：R006の非同期実行と親側の終了処理だけでは、シェル内部の生成途中を覆えませんでした。検証スクリプト内の一時資源生成箇所を再点検し、このチェッカーでは終了処理を増やす代わりにディスクへの保存をなくしました。[ADR 0040](ADR/0040-check-calendar-assets-in-memory.md)に判断を記録します。

### R015 目印確認と書き込みの通信先切り替わり

- 指摘：[3939865260](https://github.com/velengel/tsunoru/pull/6#discussion_r3939865260)
- 判断：対応が必要。DBの目印確認と書き込みが別接続では、その間に元サーバーが終了して同じポートを別プロセスが取得すると、確認済みでないDBへ書き込めます。子の生存確認を増やしても確認と送信の隙間は残ります。
- 状態：両クライアントで旧方式の誤送信を再現し、目印確認と一回の書き込みを再接続しない同じTCP接続へ固定、検証、push、返信、スレッド解決済み。
- 修正：[b2a04bd](https://github.com/velengel/tsunoru/commit/b2a04bd7a13deb8e0fe0e671eb7f11690200703a)
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939901978)
- 検証：目印応答後にリスナーを交代する負例で、旧方式の誤書き込みと修正後0件を両言語で確認。同一ソケットでの成功、ステータス・本文bytes・Cookieの保持、目印不一致拒否、通常HTTP、ブラウザー320px／1440px、元DB不変、別DB拒否、内外中断30ケースがPASS。
- 履歴：R010のDB目印は維持し、確認と利用を通信単位でも束ねます。既存の解決記録を残したまま、再接続を禁止する必要性を追加判断しました。[ADR 0041](ADR/0041-bind-verification-writes-to-one-connection.md)に採用理由と制約を記録します。

### R016 親終了後に残るプロセスグループ

- 指摘：[3939917795](https://github.com/velengel/tsunoru/pull/6#discussion_r3939917795)
- 判断：対応が必要。親の終了確認だけでは、同じグループに残る子まで終了したとは言えません。所有する子を残して親が終了するfixtureで、Harness.stop後の残存を再現しました。
- 状態：PythonとNodeで、親の生存と独立して所有グループを回収する修正を実装、検証、push、返信、スレッド解決済み。
- 修正：[0505a5b](https://github.com/velengel/tsunoru/commit/0505a5b3cac34f1b10fa0aff321f0d0a14d80a8c)
- 返信：[対応しました](https://github.com/velengel/tsunoru/pull/6#discussion_r3939955482)
- 検証：両言語で通常の子とSIGTERMを無視する子の計4ケース、Python中断6ケースと通常HTTP、Node中断14ケース、外側中断10ケース、別DB拒否2種、WAL復元と元DB不変がPASS。
- 履歴：R013の所有資源スコープを、親プロセスの終了後も残るグループへ適用します。完了した回収処理は再実行せず、終了した親の回収とグループ状態の確認を区別します。[ADR 0042](ADR/0042-reclaim-owned-process-groups-through-completion.md)に判断を記録します。

## 今回からのまとめ方
### R017 Linuxコンテナでの終了済みプロセス判定（保留）

- 指摘：[3939968590](https://github.com/velengel/tsunoru/pull/6#discussion_r3939968590)
- 判断：今回は保留。報告は、孫プロセスを回収しないPID 1を持つLinuxコンテナで、ゾンビを残存と判定して検証ツールが失敗するものです。アプリ本体の不具合ではなく、元の目的である移行後のmacOS動作確認とカレンダー修正は検証済みです。Linux対応の追加実装を今このPRで続ける必要性は低いと判断します。
- 状態：ユーザーの作業終了指示と2往復上限に従い、追加実装・再レビューは行いません。未修正のスレッドとして残します。
- 検証：macOSの既存確認はPASS。Linuxコンテナの再現は今回のレビュー報告によるもので、ローカル再検証は未実施です。
- 修正：未実施。Linuxでこの検証ツールを使う必要が生じた際の別作業候補です。
- 履歴：[ADR 0043](ADR/0043-stop-review-follow-up-after-two-rounds.md)に基づく停止時の保留判断です。R016のmacOSでの検証結果と、今回のLinux条件の報告を区別します。

## レビュー対応の停止

## R031: 実URL書き込み検証の後始末境界（2026-09-06）

- 指摘：自動レビューによる検証コマンド拒否（専用D1への合成データ書き込み後、失敗・割り込み時の削除保証が不足）。
- 判断：対応が必要。実URLで作成・回答を検証するには、イベント全体を主催者権限で削除できるAPIが必要で、検証ツールだけの管理者削除ではアプリの認可境界を確認できない。
- 状態：削除APIの失敗テスト、実装、ローカル統合検証まで完了。実URLへの再検証はWorker反映後に行う。
- 修正：このPRの削除API実装と[ADR 0059](ADR/0059-delete-staging-events-by-organizer-capability.md)。
- 履歴：先行試行は後始末の強制終了保証不足で拒否された。関連行を一つのD1 batchで削除し、作成・回答・集計・削除を同じ合成イベントで再検証する方針へ変更した。

The user subsequently stopped this loop and set a two-round limit. [ADR 0043](ADR/0043-stop-review-follow-up-after-two-rounds.md) supersedes indefinite convergence: assess relevance and impact, record defer/no-change decisions where justified, and stop after two rounds with a merge-decision report. The current loop ends without another review request; an already running review is reported as pending rather than awaited indefinitely.

今回のR010とR011は検証先の隔離という共通の観点で、一つの対応バッチにまとめます。
過去のR002、R004からR009は、終了処理を資源取得前、取得途中、起動待ち、通常動作、失敗、終了の各段階で横断確認すれば早く見つけられた可能性があります。
指摘を受けた箇所だけでなく、NodeとPythonの両方をこの観点で確認します。

今後は受信済みの全指摘を集めてから判断し、共通原因をまとめ、関連する箇所のローカルレビューと修正を収束させて一度の再レビューへ進みます。
返信は各スレッドに行いますが、再レビューは返信ごとに起動しません。
運用の根拠は[ADR 0034](ADR/0034-record-review-judgments-in-repository.md)、[ADR 0035](ADR/0035-reply-to-addressed-review-comments.md)、[ADR 0036](ADR/0036-batch-review-follow-up.md)です。

## R028-R030: Rust Worker vertical slice (2026-09-06)

- R028（セキュリティ）: **要対応（次段階）**。最小 Worker は公開 API に必要な認証・capability 検証を持たないため、ローカル検証に限定した。
- R029（エラー契約）: **保留**。D1 の重複・保存エラーは次段階の認証・匿名回答 API とまとめて設計する。
- R030（スコープ）: **変更不要**。イベント作成までの縦切りは ADR 0049 の段階的移行方針に一致する。

実装・検証: [c2145ae](https://github.com/velengel/tsunoru/commit/c2145ae)。
