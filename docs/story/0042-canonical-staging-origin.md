# Story 0042: staging origin を一元化して drift を検知する

## context

Custom Domain 移行後も README、検証コード、過去の運用証跡に旧 `workers.dev` URL が残り、次の変更で `APP_ORIGIN` と利用者向け URL がずれる可能性がある。Issue #35 の対象を、正規 URL の運用と drift 防止に限定する。

## definition of done

- 正規 staging origin と移行中 fallback の扱いを一つの設定として参照できる。
- active な README、検証手順、設定が正規 origin を示す。
- 正規 origin の変更漏れを自動検知するチェックがある。
- 旧 `workers.dev` は fallback としての終了条件が記録される。
- staging API の認証・同一 origin 挙動は変わらない。

## to do

- [x] canonical origin 定数と drift verifier を追加する
- [x] active docs と検証コマンドを canonical origin に更新する
- [x] fallback の終了条件と残存箇所を記録する
- [x] focused check と既存 worker checks を実行する

## concern

過去の実施証跡には当時の URL を残す必要がある。過去レポートを機械的に書き換えると、実際に検証した endpoint の証拠を失うため、active guidance と historical evidence を分ける。
