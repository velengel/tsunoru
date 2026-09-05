# レビュー判断履歴

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
- 状態：PythonとNodeで、親の生存と独立して所有グループを回収する修正を実装、検証済み。push後にコミットと返信リンクを追記します。
- 検証：両言語で通常の子とSIGTERMを無視する子の計4ケース、Python中断6ケースと通常HTTP、Node中断14ケース、外側中断10ケース、別DB拒否2種、WAL復元と元DB不変がPASS。
- 履歴：R013の所有資源スコープを、親プロセスの終了後も残るグループへ適用します。完了した回収処理は再実行せず、終了した親の回収とグループ状態の確認を区別します。[ADR 0042](ADR/0042-reclaim-owned-process-groups-through-completion.md)に判断を記録します。

## 今回からのまとめ方

今回のR010とR011は検証先の隔離という共通の観点で、一つの対応バッチにまとめます。
過去のR002、R004からR009は、終了処理を資源取得前、取得途中、起動待ち、通常動作、失敗、終了の各段階で横断確認すれば早く見つけられた可能性があります。
指摘を受けた箇所だけでなく、NodeとPythonの両方をこの観点で確認します。

今後は受信済みの全指摘を集めてから判断し、共通原因をまとめ、関連する箇所のローカルレビューと修正を収束させて一度の再レビューへ進みます。
返信は各スレッドに行いますが、再レビューは返信ごとに起動しません。
運用の根拠は[ADR 0034](ADR/0034-record-review-judgments-in-repository.md)、[ADR 0035](ADR/0035-reply-to-addressed-review-comments.md)、[ADR 0036](ADR/0036-batch-review-follow-up.md)です。
