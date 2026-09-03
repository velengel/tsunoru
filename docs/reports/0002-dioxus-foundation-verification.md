# Story 0002 Dioxus基盤検証記録

Date: 2026-09-01

## RED

Command: `cargo test --all-targets`

Result: expected failure.

Exit code: 101.

CargoはRustのテストcrateまでコンパイルし、まだ存在しない `tsunoru::App` のimportで失敗した。
この失敗はDioxusの依存関係やSSRテストAPIではなく、受け入れ条件を表示するアプリケーションコンポーネントが未実装であることを示している。

Observed error:

```text
error[E0432]: unresolved import `tsunoru::App`
 --> tests/app_shell.rs:2:5
  |
2 | use tsunoru::App;
  |     ^^^^^^^^^^^^ no `App` in the root
```

## GREEN

### Rustテスト

Command: `cargo test --all-targets`

Result: PASS.

Exit code: 0.

ライブラリー、実行バイナリー、統合テストをコンパイルした。
統合テスト1件は、Dioxusの `App` をHTMLへ描画し、`TSUNORU` の主見出し、プロダクト説明、`main` landmark、状態表示のaccessible nameを確認した。

### 静的検査と整形

Command: `cargo clippy --all-targets --all-features -- -D warnings`

Result: PASS.

Exit code: 0.

Command: `cargo fmt --check`

Result: PASS.

Exit code: 0.

### WebAssemblyビルド

Command: `dx build --platform web`

Result: PASS.

Exit code: 0.

Dioxus CLI 0.7.10は176個のコンパイル単位を処理し、WebAssembly、JavaScript bootstrap、HTML、CSSを `target/dx/tsunoru/debug/web/public` へ生成した。
初回のWeb buildは49.49秒で完了した。

### 開発サーバー

Command: `dx serve --platform web --open false --addr 127.0.0.1 --port 8080 --interactive false`

Result: PASS.

Dioxus CLIはアプリケーションを2.28秒で再ビルドして起動した。
`curl -I http://127.0.0.1:8080/` は `HTTP/1.1 200 OK` と `content-type: text/html` を返した。

### ブラウザー表示

Commands:

```bash
npx playwright screenshot --channel chrome --viewport-size "320, 900" --wait-for-selector h1 --wait-for-timeout 500 --full-page http://127.0.0.1:8080/ /private/tmp/tsunoru-320.png
npx playwright screenshot --channel chrome --viewport-size "1440, 1000" --wait-for-selector h1 --wait-for-timeout 500 --full-page http://127.0.0.1:8080/ /private/tmp/tsunoru-1440.png
```

Result: PASS with a stated limitation.

Chromiumは両方のviewportでDioxusの `h1` が現れるまで待ち、320×900と1440×1000のスクリーンショットを生成した。
目視では横方向の切れ、意図しない重なり、読めない文字はなく、320pxでは一列、1440pxでは二列の構成になった。
任意長の文字を置くflexまたはgridの子には `min-width: 0` と折り返し方針を設定し、装飾用カレンダーは `aria-hidden` にした。
画面に操作要素はないため、キーボード操作とfocus表示の対象はない。

アプリ内ブラウザーは、ブラウザープラグインの信頼済みパス設定で接続できなかった。
代わりにPlaywright CLIでChromiumを起動した。
ページの `scrollWidth` とconsole errorを自動取得する一時スクリプトは実行ポリシーに拒否されたため、この二点は目視と正常描画を超えて自動検証していない。

### 証拠の境界

- ローカルのtest、lint、format、Web build：PASS。
- ローカル開発サーバーのHTTP応答：PASS。
- Chromiumの320pxと1440px表示：PASS with the limitation above。
- 外部環境へのdeployment：NOT PERFORMED。
- 物理スマートフォンでの操作：UNVERIFIED。
