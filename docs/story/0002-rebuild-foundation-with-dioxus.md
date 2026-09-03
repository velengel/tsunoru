# Story 0002: DioxusでRustフロントエンド基盤を作る

Status: completed

Date: 2026-09-01

## context

React基盤のテストを先に作った後、TypeScript 7と周辺ツールの互換性を調査した。
TypeScriptを下げて互換性を得るより、Rustでフロントエンドを作りながら仕組みを理解したいという開発目的が明確になった。

Reactのアプリケーション実装はまだコミットされておらず、日程調整のドメイン実装も始まっていない。
この時点なら、既存の判断を履歴として残しつつ、小さい費用でRustフロントエンドへ切り替えられる。

## definition of done

- React、Vite、npmの基盤が、RustとDioxusのWeb基盤へ置き換わっている。
- Dioxus 0.7系の安定版を使い、画面のコンポーネントがRustで記述されている。
- ブラウザーに `TSUNORU` の見出しと、`「こうしたい」から、みんなで集まる日を決める。` という説明が表示される。
- 利用者が受け取るHTMLを検査する受け入れテストが、実装より先に追加され、期待した理由で失敗している。
- `cargo test --all-targets` が成功する。
- `cargo clippy --all-targets --all-features -- -D warnings` と `cargo fmt --check` が成功する。
- `dx build --platform web` が成功する。
- `dx serve --platform web` で開発サーバーが起動し、実ブラウザーで画面を確認できる。
- READMEにRust、Dioxus CLI、開発サーバーの起動、停止、検証方法が書かれている。
- npmの依存関係と生成物がリポジトリから除かれ、RustとDioxusの生成物がignoreされている。
- 検証結果と実装中の発見がリポジトリ内の文書に残っている。

## to do

- [x] Rustフロントエンドへの変更をStory、ADR、用語集へ記録する。
- [x] Dioxusのアプリケーションシェルに対する失敗する受け入れテストを書く。
- [x] React、Vite、npmのファイルをRustとDioxusの構成へ置き換える。
- [x] Dioxusのアプリケーションシェルとスタイルを実装する。
- [x] READMEへ開発と検証の手順を書く。
- [x] test、lint、format、Web build、秘密情報検査を実行する。
- [x] 開発サーバーを起動し、実ブラウザーで画面を確認する。
- [x] 検証結果とSurprise & Discoveryを文書へ反映する。

## concern

- Dioxusは0.x系であり、将来の更新で破壊的変更を受ける可能性がある。
- RustのコンパイルとDioxus CLIの導入は、Reactとnpmだけの構成より初回準備に時間がかかる。
- Rustで記述しても、ブラウザーのDOM、CSS、アクセシビリティ、WebAssemblyの境界を理解する必要は残る。
- サーバー機能まで同時に導入すると基盤の検証範囲が広がるため、今回はブラウザーで動く最小のWebアプリに留める。
