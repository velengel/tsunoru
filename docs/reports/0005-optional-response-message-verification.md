# Story 0005 検証記録

Date: 2026-09-02

対象commit: `d6caa04 feat: add optional post-answer messages`

## 結論

Product Story 3の実装、入力検証、回答capabilityによる認可、SQLite保存、再試行の冪等性はローカルの自動testとHTTPで確認した。
回答完了を先に伝え、ひとことは送信もskipもできる任意操作として分離した。

実ブラウザーを操作する検証は未実施である。
capabilityを保存成功とskipで破棄し、失敗時だけ保持する動的な状態遷移を含め、Storyの状態はin progressとして残す。

## test-firstの証拠

利用者向け、保存境界、responsive contractのtestは `d93a933 test: specify optional response messages` で実装より先にcommitした。

- UI、domain、server function、repositoryの未実装symbolにより、対象testはcompile errorでREDになった。
- responsive testは `.comment-offer` がないためREDになった。
- 実装レビュー中も、409の表示、network error時の `aria-invalid`、button境界色、公開projectionへの非露出、SQLiteのNUL迂回を先にtestで失敗させた。
- NULを含む直接DB書き込みは、SQLiteの `length(TEXT)` がNUL以降を数えず最初はCHECKを通過した。`instr(respondent_comment, char(0)) = 0` を追加してGREENにした。

## 自動検証

`d6caa04` で、次を実行した。

```text
cargo test --all-targets
  PASS: 36 tests

cargo test --all-targets --features server
  PASS: 57 tests

cargo test --all-targets --no-default-features --features server
  PASS: 57 tests

cargo clippy --all-targets --all-features -- -D warnings
  PASS

cargo fmt --check
  PASS

dx build --web
  PASS: client build and server build

git diff --check
  PASS
```

Story 3固有では、HTMLとdomainの8件、repositoryの6件、responsive contractの該当testがPASSした。
公開eventのJSONへ回答者名とひとことが混ざらないこともrepository testで確認した。

## HTTPとSQLite

回答済みresponseへ、次の結果を確認した。

- 有効なひとことはHTTP 200で保存された。
- Unicode空白を除いた同じ本文の再送はHTTP 200になり、一件のままだった。
- 保存済み本文と異なる再送はHTTP 409になり、先の本文を上書きしなかった。
- 空白だけの本文はHTTP 422となり、保存されなかった。

SQLiteでは、回答数とavailability数を変えず、正規化したひとことを一件だけ保存した。
生の回答capabilityは保存せず、既存のSHA-256 hashでeventとresponseを照合する。

候補版8081と検証済み版8082は、2026-09-02の確認時にどちらもHTTP 200だった。
最新migrationを実サーバーのfresh DBへ初回適用する操作はスキーマ変更の承認が得られず、隔離DBのtestだけで確認した。

## 独立レビュー

read-onlyレビューではP0とP1はなかった。
P2として、SQLiteのNUL迂回と、capability破棄・保持を実ブラウザーで直接駆動していない点が挙がった。

NUL迂回はDB CHECKとtestを追加して解消した。
後者はコード上の保存成功、skip、失敗の分岐と、子componentへcapabilityを渡さない境界を確認したが、実ブラウザー証拠はUNVERIFIEDのままである。

## 実ブラウザー

次はUNVERIFIEDである。

- 320pxとdesktopでの横overflow、二つの例文、textarea、送信、skipの見た目。
- keyboardだけで例文を選び、編集し、送信またはskipできること。
- 保存失敗後に本文が残り、再試行できること。
- 保存成功とskipでcapabilityが破棄され、失敗時だけ保持されること。
- capabilityがDOM、URL、browser storageへ出ないこと。

利用可能なin-app browser clientは起動できず、外部Playwright scriptは追加承認なしに実行していない。

## 機密情報

commit前に、既知のAWS、GitHub、OpenAI、Slack、private key形式をrepository内で検索し、一致がないことを確認した。
`.env`、秘密鍵、credential用拡張子、`secrets/`以下のtracked fileも0件だった。
SQLite、WAL、SHM、browser用一時成果物はcommitしていない。

## 証拠の境界

| 層 | 状態 | 証拠 |
| --- | --- | --- |
| Git | PASS | `d6caa04` |
| Rust test / lint / format | PASS | 統合後36件、server込み57件、Clippy、format |
| Fullstack build | PASS | client、server成果物 |
| local HTTP | PASS | 有効200、同文再送200、異文409、空白422、root 200 |
| SQLite | PASS | 認可、冪等性、競合、NULを含む直接書き込み拒否 |
| Chromium 320px / desktop | UNVERIFIED | browser client起動不良、外部script未承認 |
| 物理スマートフォン | UNVERIFIED | 実施していない |
