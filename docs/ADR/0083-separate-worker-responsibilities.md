# ADR 0083: staging Worker の横断処理と API handler を分離する

## context
Worker の入口 module に route、capability 認可の方式選択、rate limit、scheduled cleanup があり、API module にイベントと回答の保存・読取・削除処理が混在している。機能追加時に横断処理と resource 操作の変更範囲が広がり、認可境界のレビューが難しい。

## decision
route policy と resource 別 handler を module 単位で分離し、capability 認可の選択・検証を共通 policy に集約する。

## rejected options
- `lib.rs` と `api.rs` の分割を見送る: 変更範囲が広いままで、責務ごとの検証点を作れない。
- 認可を各 handler に複製する: 実装差分が生まれ、protected mutation の境界を一貫して保てない。
- API 契約を同時に変更する: リファクタの原因切り分けと既存 staging 利用の継続を難しくする。

## consequences
module 境界と内部引数が増え、短期的にはファイル数と import が増える。今回の resource module は既存 handler の再 export による機械的境界であり、handler 本体の移動は別の変更として安全に進められる。既存の status/error、D1 batch、capability 検証を保つ限り、今後の handler 変更を局所化し、レビューと構造テストを容易にできる。認可 policy 自体の仕様変更はこの ADR の対象外である。
