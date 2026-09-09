# Story 0045: staging Worker の責務を分離する

## context
`cloud/rust-worker/src/lib.rs` に route dispatch、認可方式の選択、rate limit、cleanup が集まり、`api.rs` にイベントと回答の handler が同居している。次の変更で境界を誤るコストが高いため、外部契約を保ったまま内部構造を整理する。

## definition of done
- [ ] route、request policy、event handler、response handler、cleanup が責務ごとに分離される
- [ ] capability 認可の選択と検証が共通化される
- [ ] API の status、error code、認可境界、rate limit、scheduled cleanup の挙動が変わらない
- [ ] RED テストを先に実行し、実装後に Rust/Worker 検証を通す
- [ ] Issue #37 と PR の検証結果・残件を同期する

## to do
- [ ] 構造境界を表すテストを追加する
- [ ] module 分割と共通 policy を実装する
- [ ] self-review と review 状態を記録する

## concern
module 分割の途中で認可順序や D1 batch の順序を変えると、保護された mutation や冪等性を壊す可能性がある。変更は内部 module 構造に限定し、HTTP 契約を fixture と既存試験で確認する。
