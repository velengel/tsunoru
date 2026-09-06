# Story 0030: Cloudflare の URL で限定公開版を使う

## context

ユーザーは PR #10 をマージし、少人数で使える状態まで進めるよう依頼した。入口認証と回答所有権はローカルで検証済みだが、既存の Dioxus 画面は native の server function を呼んでいる。今回は [#11](https://github.com/velengel/tsunoru/issues/11) の作成・共有・回答・主催者確認を実 URL までつなぐ。

既存 Cloudflare account の Wrangler OAuth が有効で、Koji Todo と Voice Workbench が使う account と一致する。Dioxus の browser build と Rust Worker を維持し、同じ origin の静的ファイルと API を配信する。前 PR の worktree は整理済みで、残る calendar worktree は別件のため、最新 main `4e2ae93` から `cloudflare-staging-ui` と draft PR #13 を作成した。

## definition of done

- [x] 専用 worktree と実装前の draft PR を作成する。
- [x] 試用コードで入場し、コードを静的 bundle や URL に埋め込まずに利用できる。
- [x] スマホ幅の画面で日時候補を選び、作成・共有・回答・主催者集計を完了できる（ローカル CUA で確認）。
- [x] 名前や URL だけでは主催者の一覧へ入れず、Cookie の改ざん・期限切れ・異なる Origin を拒否する。
- [x] 通信失敗後も同じイベント・回答と権限を保持して再送できる。
- [x] 日付・時刻・タイムゾーンを構造として保存し、実在しない入力を Worker でも拒否する。
- [ ] 既存アカウントの専用 Worker・新規 D1 に配置し、実 URL と配置 version の証拠を残す。
- [ ] ローカル・実 URL・320px/desktop ブラウザー・実機の証拠を区別し、必要チェックと最大2往復のレビュー判断を完了する。
- [ ] 本文・Story・ADR・判断ログを実装へ同期し、マージ可能な PR を渡す。

## to do

- [x] 既存 Cloudflare 運用、native UI と REST DTO、公式の Static Assets / Cookie / HMAC の仕様を確認する。
- [x] ADR 0055–0058 に従って失敗する HTTP / user-visible test を先に用意する。
- [x] native-fullstack と cloud-web の build を分け、カレンダー・候補操作・集計表示を共有する。
- [x] 日時 DTO とイベント作成の同内容再送を Worker / D1 に追加する。
- [x] 試用セッション、同一 origin の静的配信、ブラウザー用 REST client を実装する。
- [x] 依存固定・再現可能な build と dry-run、必須 Rust 回帰検査を実行する。
- [ ] 新規 D1、secret、staging URL を設定し、合成データで配置を検証する。
- [ ] ローカルレビューと Codex review をバッチで判断し、必要な修正・返信を行う。

## concern

ローカル画面と各検証の根拠は [report 0028](../reports/0028-staging-browser-app.md)。D1 新規作成は自動承認審査で明示承認を要求され、remote の作成と配置は未実施である。ローカル PASS と実 URL の完了を区別して残す。

今回は個人 account、回答編集、日程確定、コメント、native 履歴、旧 DB 移行を移植しない。これらの要否と一般公開向けの制限は [#12](https://github.com/velengel/tsunoru/issues/12) で判断する。試用コードは利用者別の本人確認ではなく、漏えい時は環境 secret を交換する。重要な実データの移行・継続運用を完了したとは扱わない。

検証は自身が作った D1・合成データ・プロセスに限定する。ブラウザーや公開先の強制制限が出た場合は迂回せず、実装・HTTP・画面・実機のどこまで確認できたか記録する。配置先を変更する際も既存2アプリの route、DB、認証には触れない。
