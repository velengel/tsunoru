# ADR 0003: RustフロントエンドにDioxusを使う

Status: accepted

Date: 2026-09-01

## context

ADR 0001ではReact、TypeScript、Viteを使う判断を記録した。
テスト基盤の導入中にTypeScript 7と `typescript-eslint` の互換範囲が衝突し、TypeScript 6へ下げる案を検討した。

しかし、コンパイラーの版を下げてReact基盤を維持することは、Rustフロントエンドを一度作り、ブラウザーで動く仕組みを理解したいという開発目的に合わない。
日程調整の機能はまだ存在せず、Reactの実装もコミット前であるため、技術基盤を変更できる段階にある。

## decision

- フロントエンドをRustで記述し、Web UIフレームワークには0.7系の最新安定版であるDioxus 0.7.10を使う。
- Rust 2024 Editionを使い、最低Rustバージョンは2024 Editionを利用できる1.85とする。
- 依存関係は `Cargo.toml` と `Cargo.lock` で管理し、Dioxus CLIも0.7.10へ揃える。
- アプリケーションシェルはDioxusのWebプラットフォームとしてビルドし、ブラウザーではWebAssemblyとして実行する。
- 利用者に見えるHTMLの受け入れテストにはDioxusのサーバー側レンダラーをテスト専用で使う。
- 静的検査にはClippy、整形にはrustfmtを使う。
- 今回はルーティング、サーバー関数、データベース、認証を導入しない。
- React、TypeScript、Vite、Vitest、npmの基盤は削除する。

DioxusはReactに近いコンポーネントとRSXを持つため、既に比較対象となっているReactとの違いを観察しながらRustの所有権、型、WebAssemblyの境界を学べる。
Webだけの最小構成に留めれば、日程調整の保存方式を決める前にサーバー構成を固定せずに済む。

参考資料：

- [Dioxus 0.7: Getting Started](https://dioxuslabs.com/learn/0.7/getting_started/)
- [Dioxus 0.7: Web Platform](https://dioxuslabs.com/learn/0.7/guides/platforms/web/)
- [Dioxus 0.7: Fullstack Project Setup](https://dioxuslabs.com/learn/0.7/essentials/fullstack/project_setup/)
- [The Rust and WebAssembly Book](https://rustwasm.github.io/docs/book/)

## rejected options

### ReactとTypeScript 7を継続する

TypeScript 7に対応するツールへ切り替えれば、コンパイラーを下げずにReactを継続できる。
プロダクトを早く作るだけなら有力だが、Rustフロントエンドを実際に作って学ぶという今回の目的を満たさないため却下する。

### TypeScript 6へ下げてReactを継続する

現行のReact向けツールとの互換性を得やすい。
しかし、追従していない静的検査ツールを理由にコンパイラーの本流から外れ、後で更新作業も必要になるため却下する。

### Dioxus 0.8のアルファ版を使う

新しいAPIを試せるが、基盤の再現性より先行体験を優先する理由がない。
0.7系の安定版でRustフロントエンドの学習目的を満たせるため却下する。

### Leptosを使う

LeptosもRustでWeb UIとサーバー描画を扱える有力な選択肢である。
最初のRustフロントエンドでは、Reactと比較しやすいコンポーネントモデルと公式CLIを持つDioxusのほうが、技術差を切り分けやすいため採用しない。

### 最初からDioxus Fullstackを使う

同じRustコードベースでサーバー関数とSSRを導入できる。
しかし、データ保存と配信方式が未決の基盤Storyへサーバービルドを追加すると、失敗原因と学習対象が増えるため却下する。

## consequences

- UIコンポーネント、テスト対象、将来共有するドメイン型をRustで表現できる。
- Rustの型検査、Cargo、Clippy、rustfmtを同じ開発ループで学べる。
- ブラウザー向け成果物にはWebAssemblyに加えて、読み込みに必要なHTMLとJavaScriptが含まれる。
- Dioxusと周辺ツールは0.x系であり、更新時にAPIや設定の移行が発生しうる。
- 初回ビルドとCLI導入は、既存のReact基盤より時間とディスク容量を使う。
- DOM、CSS、アクセシビリティ、ブラウザーAPIの知識は引き続き必要になる。
- JavaScript専用ライブラリを利用する機能では、Web API bindingまたはJavaScriptとの境界実装が必要になる場合がある。
- サーバー機能は別のStoryとADRで選ぶため、この基盤だけではイベントを保存または共有できない。
