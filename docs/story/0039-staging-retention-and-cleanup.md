# Story 0039: staging の合成イベントを30日で整理する

## context

Issue #28。staging を継続利用するため、合成イベントを無期限に残さず、30日を過ぎたイベントを定期処理で削除する。主催者 capability による即時削除は維持し、運用担当が定期確認できる境界を残す。

## definition of done

- staging のイベント作成時刻を保存し、30日を過ぎたイベントを scheduled cleanup で削除する。
- 候補、回答、回答選択、rate limit 行をイベント削除と整合する形で整理する。
- 主催者 capability による即時削除と、公開イベント・匿名回答の通常導線を壊さない。
- disposable D1/Worker で期限前保持、期限超過削除、再実行、関連行削除を検証する。
- staging 合成データだけを対象とし、production データの受け入れ条件とは分離する。

## to do

- [x] 作成時刻列と scheduled cleanup を追加する
- [x] 期限・再実行・関連行のテストを追加する
- [x] ADR、README、検証記録を更新する

## concern

既存の古い行には作成時刻がない可能性があるため、migration では現在時刻を補完し、補完直後の自動削除を避ける。誤削除からの復元は backup/restore Issue #29 で扱う。
