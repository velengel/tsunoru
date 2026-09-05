# Story 0028: 分離 staging のイベント・匿名回答 API

## context

PR #8でRust WorkerとD1のローカル成立性を確認した。次は本番資源を触らず、分離staging向けのAPI契約とUI接続を小さく実装する。

## definition of done

- [x] 分離staging用のD1 schemaとWorker APIを定義する。
- [x] 主催者用 capability と回答用 capability の境界をサーバー側で検証する。
- [ ] イベント作成と匿名回答をDioxus browser UIから操作できる。
- [ ] 320px幅・キーボード操作・API異常系をローカルで検証する。
- [ ] 本番資源を使わず、デプロイ手順と未実施事項を記録する。

## to do

- [x] API契約と認証境界のADRを作成する。
- [x] WorkerのD1 schema、capability、イベント作成、匿名回答を実装する。
- [ ] Dioxus UIをWorker APIへ接続する。
- [ ] 分離fixtureを使ったWorker/browser検証を追加する。
- [ ] self-reviewと判断ログを更新し、PRをreadyにする。

## concern

本番D1、secret、ドメインは変更しない。認証を迂回する公開endpointや、既存SQLiteとの自動同期は今回の範囲外とする。
