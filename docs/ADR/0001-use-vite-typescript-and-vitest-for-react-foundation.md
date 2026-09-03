# ADR 0001: Vite、TypeScript、Vitest で React 基盤を作る

Status: superseded

Date: 2026-09-01

Superseded by [ADR 0003](0003-use-dioxus-for-the-rust-web-foundation.md) after the user selected a Rust frontend.

## context

空のリポジトリに、次の機能開発へ進める React の最小基盤が必要である。
現時点ではルーティング、サーバー描画、データ取得、永続化の要件がなく、フルスタック構成を決める根拠はない。

React 公式文書は、要件に合う framework がまだ定まらない場合の build tool として Vite を含む選択肢を示している。
Vite は開発サーバーと production build を提供し、React と TypeScript の template を公式に用意している。
Vitest は Vite の設定を共有でき、React Testing Library は内部実装ではなく利用者が触れる DOM を基準にテストできる。

## decision

- React と TypeScript で client-side の single-page application を作る。
- Build tool と開発サーバーには Vite を使い、package manager には npm と `package-lock.json` を使う。
- テスト runner には Vitest、DOM 環境には jsdom、component の検証には React Testing Library と `jest-dom` matcher を使う。
- 最初の受け入れテストは、role と accessible name から見出しを探し、利用者に見える説明を確認する。
- Lint には TypeScript と React Hooks を検査する ESLint flat config を使う。
- ルーター、外部状態管理、API client、CSS framework は、必要性を示す Story と ADR ができるまで導入しない。
- Node.js の要件は Vite の現行要件に合わせ、ローカルで確認できた Node.js 24 系を開発基準にする。

この構成なら、Vite と Vitest が解決規則を共有し、最小の依存関係でテスト先行の React 開発を始められる。
未決のプロダクト構造を埋めないことも、今回の判断に含む。

参考資料：

- [React: Build a React app from Scratch](https://react.dev/learn/build-a-react-app-from-scratch)
- [Vite: Getting Started](https://vite.dev/guide/)
- [Vitest: Getting Started](https://vitest.dev/guide/)
- [Testing Library: React Testing Library](https://testing-library.com/docs/react-testing-library/intro/)

## rejected options

### Create React App

React 公式文書で deprecated とされており、新規基盤として採用しない。

### 最初から React framework を採用する

Framework は routing、data fetching、rendering 戦略を統合できる。
しかし、その要件がない段階で採用すると、基盤だけの Story に不要な構造と依存関係を持ち込むため却下する。

### JavaScript だけで始める

初期ファイル数はわずかに減るが、component の契約と test code の型検査を後から移行する負担が生じるため却下する。

### Jest を独立して構成する

成熟した選択肢ではあるが、Vite と別の変換設定を持つ理由が現時点になく、同じ設定を利用できる Vitest より構成が増えるため却下する。

## consequences

- 開発サーバー、production build、unit test が npm scripts で揃う。
- TypeScript と利用者視点の DOM test が、次の実装の安全網になる。
- npm の lockfile によって、直接依存と間接依存の解決結果を再現できる。
- Node.js の version が下限未満の環境では、依存関係の導入や開発サーバーの起動に失敗する。
- client-side SPA だけでは SSR、SSG、React Server Components、route 単位の data fetching を提供できない。
- jsdom は layout、描画、実 browser 固有の挙動を再現しないため、見た目や interaction の証拠には別の browser test が必要になる。
- 将来 routing や server rendering が必要になれば、framework 採用を後続 ADR で判断し、移行コストを飲み込む。
