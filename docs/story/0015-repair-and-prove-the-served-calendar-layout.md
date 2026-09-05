# Story 0015: 配信中の候補日カレンダーを直して実ブラウザーで確かめる

Status: complete

Date: 2026-09-02

## context

候補日カレンダーを表示した画面で、前月button、年月、次月buttonが縦に離れ、曜日と日付も七列にならず横へ流れた。
sourceのCSSにはtoolbarと七列gridがあるが、`127.0.0.1:8081` のHTMLが参照したhash付きstylesheetには、そのselectorが含まれていなかった。

同じworktreeで複数の `dx serve` を長時間動かし、同じ `target/dx` を共有していた。
新しいSSR markupと古いstylesheetが組み合わさっても、source assertionとFullstack buildだけでは検出できない。

## definition of done

- 既存の修正branchとworktreeを継続し、main向けPRとして検証済みの変更を渡す。
- Dioxusが配信するHTMLからstylesheet URLを取得し、その実fileにcalendar固有selectorが含まれることを自動検査する。
- 候補サーバーは既存serverとbuild出力、session cache、portを共有せず、同じ世代のSSR、Wasm、CSSを配信する。
- 前月button、年月、次月buttonを一つのtoolbarへ置き、曜日と日付を七列へ揃える。
- `19:00` の文字input、日付の追加と解除、前月と次月への移動が実ブラウザーで動く。
- 320pxと1440pxでpage-level横overflowがなく、calendarの日付targetが24px四方以上ある。
- source、live DOM、配信stylesheet、computed style、geometry、screenshotを別々の証拠として残す。
- calendarからeventを作り、回答送信後に「みんなの回答」が表示される一続きの操作を確認する。
- 画面崩れを再現する失敗するtestを実装より先に追加する。
- test、Clippy、format、Fullstack build、秘密情報検査が成功する。

## to do

- [x] 崩れた画面とsource、配信HTML、配信stylesheetを比較する。
- [x] Dioxus asset、CSS Grid、WCAG Reflow、target size、ARIA Gridの公式資料を確認する。
- [x] current featureをlocal mainへfast-forwardし、修正branchを作る。
- [x] 配信assetと実ブラウザーを測るuser-level skillを作る。
- [x] server分離と証拠の境界をADRへ記録する。
- [x] 配信stylesheetの世代不一致を再現する失敗するtestを書く。
- [x] 分離した候補サーバーでSSR、Wasm、CSSを同じbuildから配信する。
- [x] 320pxと1440pxでcomputed layoutとscreenshotを確認し、必要なCSSを直す。
- [x] calendar選択から回答後一覧まで実ブラウザーで操作する。
- [x] README、Story、検証記録、Surprise & Discoveryを更新する。

## concern

- source CSSを直すだけでは、browserが古いhash付きassetを読んだ問題を再発防止できない。
- 複数の `dx serve` が同じ出力先へ書くと、portを分けてもassetの世代は分離されない。
- target directoryを分けると初回build時間とdisk使用量が増える。
- calendarを七列へ戻しても、320pxでは一日あたりの幅が約25pxであり、44px四方にはできない。WCAG 2.2の24px四方を下限にする。
- 自動化したChromium操作はscreen readerでの読み上げを証明しない。

## 2026-09-05 resumption plan

Migration is complete. Resume in the existing `fix/calendar-layout-verification` worktree and branch. The briefly created PR #5 is superseded; its planning work is preserved here.
The local-main integration step above is historical; this run finishes through a feature PR.

1. Add a failing live browser assertion for seven tracks, one toolbar row, 24px targets and page overflow; record unchanged-CSS evidence.
2. Add a stale-stylesheet negative fixture, then apply the preserved narrow-calendar repair.
3. Build into this worktree's own target directory. Use an owned server and disposable database for browser writes.
4. Verify 320px and 1440px, pointer and keyboard, and calendar-to-answer completion with screenshots.
5. Run all required checks. Review the complete PR diff, fix findings, repeat affected checks and mark ready after convergence.

In-app browser bootstrap fails before browser selection with a trusted dependency path error.
Use an independent installed Playwright Chromium test runner without changing plugin trust or global permissions.

Verification: [browser report](../reports/0019-calendar-browser-verification.md). All implementation and local verification steps are complete; merge and deployment remain separate operations.
