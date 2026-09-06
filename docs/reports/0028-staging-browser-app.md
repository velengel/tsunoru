# Cloudflare 限定試用版の検証

2026-09-06、[PR #13](https://github.com/velengel/tsunoru/pull/13)、Story 0030。
作成、共有、回答、主催者集計までを、Dioxus CSR と Rust Worker の同一 origin で動かした。
Cloudflare の専用 staging Worker は配置済みである。実 URL の read-only 検証は通ったが、リモート D1 へ合成イベントを書き込む検証は自動審査により実施していない。

## 実装の境界

`native-fullstack` を既定 feature として維持し、`cloud-web` で CSR だけを選ぶ。
カレンダー、候補日時の編集、回答表は `src/shared_ui.rs` を共有し、限定版の画面と REST 呼び出しは `src/cloud/` に置いた。
既存 account、履歴、コメント、日程確定、端末間回復は移植していない。

Worker は構造化した日時と IANA zone を検証し、専用 D1 に保存する。
作成前にブラウザーへ ID、主催者権限、送信内容を保存し、同じ内容の再送だけを200で受け付ける。
回答も回答単位の権限と完全な候補集合を照合する。
試用コードは12時間の署名付き Cookie に交換し、主催者権限とは分ける。
詳細は ADR 0055–0058 と [Worker の手順](../../cloud/rust-worker/README.md) を参照。

## 検証結果

| 層 | 結果 | 根拠 |
| --- | --- | --- |
| 実装前の失敗 | PASS | 旧 Worker は新しいイベント POST を400、ログイン POST を401、静的 root を404で返した。新しい期待値はそれぞれ201、200、200。旧 UI に試用コードがない SSR 試験も失敗 |
| 既存アプリ | PASS | `cargo test --all-targets`、同 `--features server`、all-features Clippy、`cargo fmt --check`、`dx build --web` |
| CSR | PASS | `cargo test --no-default-features --features cloud-web --test cloud_journey` の9件、Wasm target Clippy、release CSR build |
| Worker | PASS | `npm run check`、Wasm target check/Clippy、fmt。認証、Origin、日時、同時再送、競合、batch rollback、API-before-SPA、SIGINT/SIGTERM 後の回収 |
| 配置準備 | PASS | `npm run deploy:check`。CSR の6ファイルと Rust Worker を新しく生成し、remote upload 前に終了 |
| ローカルブラウザー | PASS | CUA でログイン、作成、URL コピー、全候補回答、主催者集計、再読込、入力訂正、退場を操作 |
| Cloudflare 実 URL | PARTIAL PASS | health 200、静的 root 200、未認証 API 401。Worker version `98c768d7-a291-4c24-9224-b8587bff79b1`、startup 7 ms |
| Cloudflare 実 URL の書き込み journey | UNVERIFIED | 合成データを remote D1 に書き込む検証は、cleanup の確実性を理由に自動審査で拒否された |
| 実物のスマホ、支援技術 | UNVERIFIED | viewport 検証を実機やスクリーンリーダーの証拠とは扱わない |

CSR の確定コマンドは `dx build --web --release --no-default-features --features cloud-web --debug-symbols=false`。
CLI が最適化エラーを記録しても終了コード0となる試行があったため、コードだけで成功を判断しなかった。
デバッグ情報を除いた再試行では最適化エラーがなく、実際の Wasm 画面操作も通った。

初回 hosted review の [生成物回収の指摘](https://github.com/velengel/tsunoru/pull/13#discussion_r3942568289) は、失敗したビルドが正常な配置候補へ混ざるため修正した。
合成 CLI で Worker が途中の `index.js` を書いて失敗させ、既存 bundle にそのファイルが残る失敗を先に確認した。
修正後は Worker と `build/public/` を同じ一時 bundle で完成させてから切り替える。
失敗、SIGINT、SIGTERM、成功の4ケースで、直前の完成物の保全、成功時の更新、所有した一時出力と子プロセスの回収を確認した。
実際の `npm run deploy:check` と Worker の全 HTTP 試験も再実行して通った。アプリの6ファイルは修正前と一致し、変更は配置用の生成手順に限られる。

2往復目の [補助プロセスの指摘](https://github.com/velengel/tsunoru/pull/13#discussion_r3942598160) は、失敗したコマンドの子だけが生き残る合成ケースで再現した。
コマンドが終了しても所有 group の PID を保持し、失敗時は TERM と期限付きの待機、必要時の KILL で回収する。継承した pipe の終了待ちは、その回収後に行う。
TERM を無視する補助プロセスも含めた4ケースと実際の dry-run が通り、6ファイルの hash と一時ディレクトリの消滅を確認した。
実 CLI での同じ残留は未観測だが、既存の後片づけ要件へ小さく対応できるため修正した。これで2往復を終え、最終修正の追加 hosted review は行わない。

## ブラウザーで観測したこと

専用 worktree の `http://localhost:8791` を使用し、ローカル D1 と合成データだけを操作した。
2026年9月10日と11日の19時を選び、名前「検証参加者」で○と△を送った。
主催者集計には1人と二つの回答が表示され、再読込しても回答済みの状態を保持した。
共有 URL のコピーも一致した。

回答漏れでは、未回答の候補へフォーカスが移った。
Space と矢印キーで回答を選び、Enter で保存できた。
フォーカス枠は3pxで表示され、集計表は矢印キーで `scrollLeft = 40` へ移動した。
不正な `Invalid/Zone` に対する400の後は入力フォームへ戻り、イベント名と候補日を保持したまま訂正して作成できた。
退場後の再読込では再び試用コード入力になった。

| 画面 | viewport | 文書の client / scroll 幅 | 操作部分 |
| --- | --- | --- | --- |
| 作成 | 320px | 305 / 305px | カレンダー7列、gap 2px、日ボタン25.91 × 44px、選択マークあり |
| 回答 | 320px | 320 / 320px | ○△×のラベル69.60 × 51.91px、3列 |
| 集計 | 320px | 305 / 305px | 表474.22pxを273pxの領域内で横スクロール |
| 作成 | 1440px | 1425 / 1425px | カレンダー7列、gap 3.2px、日ボタン66.19 × 44px |
| 集計 | 1440px | 1440 / 1440px | 表1038pxが1040pxの領域内に収まる |

320px でスクロールバーが表示されると有効幅は305pxになる。
最初は `html` と `body` の最小幅320pxが15pxのはみ出しを生み、body だけの修正では残った。
限定版の CSS で両方を解除し、再生成したファイルで測定とスクリーンショットを確認した。
CSS 文字列の一致だけを確認する試験は採らず、この実測を見た目の根拠とする。

実際に読み込んだ CSS は `assets/cloud-dxh7745da52a7cfcfd7.css`。
HTTP 200、`text/css; charset=utf-8`、SHA-256 `a9bdbe2d945cc335f892341d08386144399ce7f0d959e107299ea350b501cc15` が生成物と一致した。
レスポンスには CSP、nosniff、no-referrer、frame 拒否を確認した。
操作後の browser error/warn は0件で、CSP違反も観測していない。
取得した CSS とヘッダーは ignored `.mydocs/cloudflare-staging-ui/`、ビルドの hash 一覧は `cloud/rust-worker/build/asset-sha256.txt` に保存した。

検証タブは閉じ、viewport override を解除した。
検証用 Wrangler は終了コード0を確認して停止した。
ローカル D1 の合成データは、この worktree の ignored `.wrangler/state/` に診断用として保持する。

## 配置の残件

既存アカウントで Wrangler OAuth が有効なことを確認した。
D1 一覧には既存2アプリだけがあり、`tsunoru-staging` Worker も存在しなかった。
予定する専用 D1 は `tsunoru-staging`（APAC hint）、Worker も同名、URL は `https://tsunoru-staging.kounakadora528.workers.dev`。

専用 D1 `tsunoru-staging`（ID `28a6410c-bf8b-4f93-9b22-5fcd77c15cd4`、APAC）を作成し、空であることを確認して fresh schema を一度適用した。
試用コードはランダムな64文字hexを secret として初回 deploy に渡し、値は表示・保存・commitしていない。Worker と6つの assets の配置は成功し、URL は `https://tsunoru-staging.kounakadora528.workers.dev` である。
read-only の実 URL 検証（health、root、未認証 API）は通った。書き込み journey は合成データの cleanup を保証する検証スクリプトが自動審査で拒否されたため未実施である。
既存2アプリの DB、route、認証は対象外である。

少人数の試用以降に必要な制限、保持と削除、復元、個別認証は [#12](https://github.com/velengel/tsunoru/issues/12) で要否を判断する。
実装があることを理由に一般公開や実データ移行まで完了したとは扱わない。

## 調査に使った一次情報

- [Static Assets の Worker routing](https://developers.cloudflare.com/workers/static-assets/routing/worker-script/)：同じ origin の assets と API を配信し、API の認証を先に実行する根拠。
- [Cookie の属性](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie)：HttpOnly、Secure、SameSite と host prefix の境界。
- [RustCrypto HMAC](https://docs.rs/hmac/0.12.1/hmac/)：署名と定時間の検証 API。試験では Node の独立した HMAC とも照合した。
- [Chrono の曖昧な日時](https://docs.rs/chrono/latest/chrono/offset/type.MappedLocalTime.html)：DST gap/fold を拒否する判断の根拠。
- 固定版 Dioxus の `dioxus-web-0.7.10/src/document.rs`：`document::Title` が内部で eval を使うことを確認し、静的タイトルを利用した。
