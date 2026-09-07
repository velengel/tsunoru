# Story 0037: Rust設計レビュー助言スキルを共有する

## context

Rust、Dioxus、Tokio、SQLx、Cloudflare Workersの設計・実装・レビューで、公式資料と一次情報に基づく助言を再利用できるようにする。

## definition of done

- `rust-expert`スキルが構造検証を通過する。
- StoryとADRに用途、判断、制約、リスクを記録する。
- UI起動文から明示的に`$rust-expert`を呼び出せる。

## to do

- [x] スキル本体とUI metadataを追加する
- [x] 公式資料優先とレビュー判断契約を記録する
- [x] Issue #17以降の設計・レビューで利用する

## concern

公式資料が存在しない論点では一次フォーラムを補助的に使う。スキルはレビューの判断を補助するもので、ユーザーのスコープや認可を拡張しない。
