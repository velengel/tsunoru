# Story 0003 検証記録

Date: 2026-09-01

対象commit: `183eb92 feat: create and share anonymous events`

## 結論

Product Story 1の完了条件は、ローカル開発環境で満たした。
ログインしていない主催者がイベントを作り、主催者権限を含まない共有URLを別のbrowser contextで開ける。
SQLiteへ保存したイベントはFullstack serverを停止して起動し直した後も残った。

外部環境へのdeploymentと物理スマートフォンでの操作は、この記録の対象外である。
320pxのChromium検証は実施したが、実機OSのdate/time pickerを操作した証拠とは区別する。

## test-firstの証拠

実装前の利用者向け受け入れテストは `db0901c test: specify anonymous event creation`、保存境界のテストは `93b620a test: specify event persistence` でcommitした。
前者は `domain` と `ui` が存在しないため、後者は `server` featureとrepositoryが存在しないため、それぞれ期待どおりREDになった。

独立レビュー後の修正でも、実装より先に次のREDを確認した。

- `CreationSuccess` に復旧キーの状態がなく、`cargo test --test event_creation` がcompile errorになった。
- repositoryの作成結果が `()` であり、保存済みaggregateを返すtestがcompile errorになった。
- 補助文字の低コントラスト色と、日本語のdocument shellがないtestが失敗した。
- 入力上限の定数とひとこと用errorがなく、入力上限testがcompile errorになった。
- server buildが `Fake/Zone` を受理し、IANAタイムゾーン検証testが失敗した。
- 未知の `/events/{public_id}` がHTTP 200を返した。

## Rustの自動検証

次のコマンドは実装commit直前に成功した。

```text
cargo test --all-targets
  PASS: 16 tests

cargo test --all-targets --features server
  PASS: 20 tests, including 3 repository tests and 1 server time-zone test

cargo test --all-targets --no-default-features --features server
  PASS: 20 tests

cargo clippy --all-targets --all-features -- -D warnings
  PASS

cargo fmt --check
  PASS

dx build --web
  PASS: client build and server build
```

通常testでは、HTML、入力必須、任意のひとこと、複数候補、重複、入力上限、復旧キー、focus、document language、responsive CSSを検査した。
server featureでは、migration、transaction rollback、候補順序、capabilityのSHA-256 hash、IANAタイムゾーンを追加で検査した。

## 実ブラウザー

PlaywrightとChromiumで、次を確認した。

- 320×900で、空送信の日本語error、イベント名、任意のひとこと、二つの候補日時を入力した。
- 作成後の共有URLにcapabilityがなく、生のcapabilityが画面へ出ない通常経路を確認した。
- clipboardへのURLコピー、別contextでの公開画面、reload、候補の入力順、横overflowなしを確認した。
- 1440×1000で、主催者のひとことを省略して作成でき、横overflowがないことを確認した。
- 最初の作成requestを失敗させ、name、ひとこと、未追加の候補日時が残り、そのまま再試行できることを確認した。
- pointerを使わず、Tabで候補追加と送信へ移り、Enterで作成し、成功見出しへのfocus、URLコピー、共有URL遷移まで完了した。
- `localStorage.setItem` を `SecurityError` にした場合、64桁の復旧キーが一度だけ表示され、共有URLへ混ざらないことを確認した。

一式の結果は5件PASSだった。
復旧パネルを追加した後、その経路を単独でも1件PASSとし、320pxのfull-page screenshotを目視した。

確認画像：

- `/private/tmp/tsunoru-story1-mobile-form.png`
- `/private/tmp/tsunoru-story1-mobile-success.png`
- `/private/tmp/tsunoru-story1-mobile-public.png`
- `/private/tmp/tsunoru-story1-mobile-recovery.png`
- `/private/tmp/tsunoru-story1-desktop-form.png`
- `/private/tmp/tsunoru-story1-desktop-success.png`

画像では、横方向の切れ、意図しない重なり、操作不能なcontrolを認めなかった。
小さい補助文字は、独立レビューで4.5:1を下回った色を変更し、主な明色背景に対して5.36:1以上となる色へ揃えた。

## server再起動とHTTP

候補版を8081、検証済み安定版を8082で起動した。
8082がHTTP 200であることを確認してから8081を停止し、8081がconnection failure、8082が引き続きHTTP 200であることを確認した。

8081を起動し直した後、再起動前に作成した次の共有URLをfresh browser contextで開いた。

```text
/events/fb2f6640-3ecf-4578-b689-04ccaf1cff97
```

イベント名と候補日時を確認するPlaywright testは1件PASSだった。
同じdocumentで `html lang=\"ja\"` と `robots: noindex, nofollow` も確認した。

HTTPを直接確認し、rootと既存eventは200、未知のevent IDは404となった。
検証後も候補版8081は起動した状態にしている。

## SQLiteと秘密情報

再起動後の `var/tsunoru.sqlite3` には18イベント、21候補があった。
保存されたcapability hashは全件64文字で、`PRAGMA foreign_key_check` は行を返さなかった。

`.gitignore` により、DB本体、WAL、SHMはすべて `/var/` の規則で除外された。
commit前に次を確認した。

- `git diff --cached --name-status`: 実装、test、migration、対応文書だけ。
- `git diff --cached --check`: PASS。
- `.env`、private key、credential用拡張子を持つstage file: 0件。
- AWS、GitHub、OpenAI形式、private key、値を代入したAPI key、access token、passwordのstaged diff一致: 0件。

DB、browser storage、Playwrightの一時成果物はcommitしていない。

## 独立レビュー

read-onlyの実装レビューと検証監査を並行して実施した。
主な指摘は、localStorage保存失敗による主催者権限喪失、commit後の再読込による重複作成の窓、入力上限、実在するタイムゾーン検証、成功時focus、文字コントラスト、document language、検索非公開性、HTTP 404だった。

Story 1の範囲で指摘を反映し、testと実ブラウザーを再実行した。
networkがcommit後の応答だけを失う場合の厳密な冪等性は残るため、公開運用で必要になった時点でrequest IDを判断する。

## 証拠の境界

| 層 | 状態 | 証拠 |
| --- | --- | --- |
| Git | PASS | `183eb92` |
| Rust test / lint / format | PASS | 16件、server込み20件、Clippy、format |
| Fullstack build | PASS | client、server成果物 |
| local HTTP | PASS | 8081で200、未知eventは404 |
| Chromium 320px / 1440px | PASS | 作成、共有、再試行、keyboard、復旧、overflow |
| server再起動 | PASS | 8082維持中に8081を再起動し、同じ共有URLをfresh contextで確認 |
| 外部deployment | UNVERIFIED | 実施していない |
| 物理スマートフォン | UNVERIFIED | 実施していない |
