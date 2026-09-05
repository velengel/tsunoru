# ADR 0046: Cloudflare互換性を独立した実験crateで測る

## context

現行serverはファイルSQLiteとnative依存を持つ。
全面移植してから失敗するリスクを避け、Dioxus、保存、認証の障害を分離して測る。
worker/worker-build 0.8.5を公開registryで確認した。

## decision

Cloudflareの小実験は本体から分離したcrateと使い捨てlocal D1で行う。

## rejected options

- 本体を先に書き換える。移植可否を知る目的には変更量が大きい。
- SQLのモックだけを使う。workerd/D1境界を検証できない。
- 合成の実験APIを本番公開する。認証を省略した検証口を公開する必要はない。

## consequences

本体の依存と挙動を保てるが、実験成功は製品の移植完了を意味しない。
実験用schemaとfixture、Argon2固定saltは合成データ専用とし本番に採用しない。
依存と生成物を実験directoryへ閉じ込め、local-only設定を使う。
