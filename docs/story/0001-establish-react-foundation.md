# Story 0001: React 開発基盤を整える

Status: superseded

Date: 2026-09-01

Superseded by [Story 0002](0002-rebuild-foundation-with-dioxus.md) after the user selected a Rust frontend.

## context

`tsunoru` は、まだファイルも Git 履歴もない新規プロジェクトである。
最初の機能を安全に積み上げるには、React の画面が起動するだけでなく、Story、ADR、テスト、用語、秘密情報の境界が同じ履歴に残る必要がある。

一方、プロダクトの用途、画面遷移、データ保存先は決まっていない。
今回の基盤は、未決の要件を先回りせず、次の Story が実装を始められる最小範囲に留める。

## definition of done

- Git リポジトリが `main` ブランチで初期化されている。
- ルートの `AGENTS.md` に Story-first、ADR、テスト先行、用語管理、commit 形式、秘密情報保護が記録されている。
- TypeScript で書かれた React のアプリケーションシェルが、Vite の開発サーバーで起動する。
- 画面にプロジェクト名 `tsunoru` と、開発基盤の準備が完了したことを示す説明が表示される。
- 利用者が認識できる見出しと説明を、React Testing Library のテストが確認する。
- 実装前にテストを実行し、期待した理由で失敗した記録が残る。
- `npm test`、`npm run lint`、`npm run build` が成功する。
- README に必要環境、依存関係の導入、開発サーバーの起動、停止方法が書かれている。
- `.gitignore` と commit 前検査によって、環境ファイルや鍵を commit 対象から外せる。
- 採用した技術判断と初期用語が、それぞれ ADR と用語集に記録されている。

## to do

- [x] Git を `main` ブランチで初期化する。
- [x] リポジトリ規約、Story、ADR、用語集、秘密情報の除外規則を作る。
- [x] テスト基盤と、アプリケーションシェルの失敗するテストを追加する。
- [ ] React のアプリケーションシェルを実装する。
- [ ] README に開発サーバーの起動方法を書く。
- [ ] test、lint、build、秘密情報検査を実行する。
- [ ] 検証結果と発見を文書へ反映する。

## concern

- プロダクト要件が未確定のため、ルーティング、状態管理、通信、永続化を今決めると不要な制約になりうる。
- Vite とテストツールの現行版は Node.js の下限を持つため、README と package metadata の要件を一致させる必要がある。
- jsdom のテスト成功は実ブラウザでの表示を保証しないため、画面仕様が増えた段階でブラウザ検証を別に設計する必要がある。
- ignore 規則だけでは、別名の秘密情報やソースへ直書きされた token を防げないため、commit 前の staged diff 検査も欠かせない。
