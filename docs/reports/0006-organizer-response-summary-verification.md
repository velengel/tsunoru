# Story 0006 検証記録

Date: 2026-09-02

対象commit: `1a4ce5d feat: add organizer response summary`、`efd63a6 refactor: expose organizer snapshot phases to tests`

## 結論

Product Story 4の主催者専用回答サマリーは実装済みである。
主催者capabilityで認可した後、回答件数、全候補の○、△、×、控えめな判断補助ラベル、最大3件のひとことpreviewを同じSQLite snapshotから返す。

自動test、file-backed WAL、Fullstack build、候補版HTTPはPASSした。
実ブラウザーで320pxとdesktopを操作する検証は未実施のため、Storyの状態はin progressとして残す。

## test-firstの証拠

利用者向けHTML、repository、server function、responsive contractは `67dcf58 test: specify organizer response summary` で実装より先にcommitした。

- domain、UI、storage、serverの未実装symbolにより、対象testはcompile errorでREDになった。
- responsive testは `.organizer-summary-page` がないためREDになった。
- コメント表示のreviewでは、`dangerous_inner_html` を禁止する回帰testを先にREDにし、通常のDioxus text nodeへ戻してGREENにした。
- 判断補助ラベルは、優先順位、○の単独最多、最多同数を追加testで固定した。同率候補を勝者として扱わない。

独立レビューで不足が見つかったWAL snapshot testは `3acf99a test: verify organizer summary snapshot` で先にcommitした。認可とprojectionを分ける内部境界が未実装のためcompile errorでREDになり、`efd63a6` でGREENにした。

## 自動検証

統合後に次を実行した。

```text
cargo test --all-targets
  PASS: 45 tests

cargo test --all-targets --features server -- --test-threads=1
  PASS: 78 tests

cargo test --all-targets --no-default-features --features server -- --test-threads=1
  PASS: 78 tests

cargo clippy --all-targets --all-features -- -D warnings
  PASS

cargo fmt --check
  PASS

dx build --web
  PASS: client build and server build

git diff --check
  PASS
```

Story 4固有では、UIとdomainの8件、repositoryの6件、server境界の5件、file-backed WAL snapshotの1件、responsive contractの該当testがPASSした。

直前の並列実行では、既存の `simultaneous_identical_retries_create_one_response` が一度だけ60秒を超えて停止したため、そのprocessを終了した。同test単独と、一列実行したserver全78件は直後にPASSし、以前の並列全件でもPASSしている。再現条件と原因は未確定であり、並列testの一時停止として検証上の懸念に残す。

## 認可と機密情報

主催者専用APIはPOSTで受け付ける。生の主催者capabilityを64文字の小文字16進数として検証し、SHA-256 hashへ変換した後に破棄する。
repositoryはevent IDとhashの組を最初のSELECTで照合し、不在event、誤った値、別eventの値を同じ404へ変換する。

APIのtyped projection、公開event projection、SSR HTMLには、生のcapability、hash、内部response IDを含めない。
入力型のDebug表示はcapabilityを `[REDACTED]` にする。
private APIは検証より先に `Cache-Control: no-store` を設定する。

候補版8081へ架空入力を送り、形式不正の422と存在しない組の404がどちらも `Cache-Control: no-store` を返すことを確認した。
成功応答は実在する秘密値をHTTP検証へ使わず、隔離DBのserver testで確認した。

## SQLite snapshot

認可、回答数、候補集計、コメント件数、previewを一つのDEFERRED read transactionで読む。
各候補の三値合計が回答数と違う場合、候補が欠ける場合、コメント件数が回答数を超える場合はprivate projectionを返さない。

productionと同じfile-backed WAL、最大5 connectionのtestでは、次の順序を固定した。

1. reader transactionがevent IDとorganizer hashを照合し、snapshotを確立する。
2. 別connectionが一件の回答をcommitする。
3. 進行中のreaderは回答0件、候補の○0件を返す。
4. 新しいtransactionは回答1件、候補の○1件を返す。

これにより、認可時点と集計時点の状態を混ぜないことを実測した。

## 表示境界

- 回答0件では全候補を0、0、0で残し、空集合を「全員」と表現しない。
- 判断補助は記録済みの件数から導く四つの事実だけとし、score、順位、推薦、日程決定を表示しない。
- 候補は作成順を保ち、desktopでは二列、320px契約では一列にする。
- ひとことは閉じたnative `details` に最大3件だけ置き、通常のtext nodeとして描画する。
- 復旧キー入力はpassword fieldとし、成功した値だけをevent別のlocalStorage keyへ保存する。
- 再読込に失敗しても、表示中のサマリーを消さない。

## 独立レビュー

domain、storage、serverを担当外のagentがread-onlyで確認し、P0とP1はなかった。

P2として、WALの途中commitを挟むsnapshot testと判断補助ラベルの分岐coverageが不足していた。どちらも追加testで解消した。
HTTP levelの `no-store` は422と404を実測したが、自動回帰testはsource契約と固定Dioxus 0.7.10のcontext testに分かれている。将来custom routerへ移す場合は、middlewareまたはrouter-level testへ引き上げる。

## 実ブラウザー

次はUNVERIFIEDである。

- 320pxとdesktopでの候補カード、三値件数、長いevent名、長いひとことのoverflow。
- keyboardだけでコメントを開閉し、再読込と復旧キー入力を操作できること。
- 回答0件、複数回答、誤った復旧キー、通信失敗からの再試行の動的な状態遷移。
- 回答者contextからprivate集計を読めず、DOM、URL、browser storage、consoleへ秘密値が出ないこと。

利用可能なin-app browser clientは起動できず、外部Playwright scriptは追加承認なしに実行していない。

## 機密情報

commit前に、既知のAWS、GitHub、OpenAI、JWT、private key形式をrepository内でファイル名だけ返す方法で検索し、一致がないことを確認した。
SQLite、WAL、SHM、退避DBは `var/` にあり、Git対象外である。

## 証拠の境界

| 層 | 状態 | 証拠 |
| --- | --- | --- |
| Git | PASS | `1a4ce5d`、`3acf99a`、`efd63a6` |
| Rust test / lint / format | PASS | default 45件、server 78件、Clippy、format |
| Fullstack build | PASS | client、server成果物 |
| local HTTP | PASS | 候補版8081と安定版8082のroot 200、private 422/404のno-store |
| SQLite | PASS | 認可、三値集計、不変条件、file-backed WAL snapshot |
| Chromium 320px / desktop | UNVERIFIED | browser client起動不良、外部script未承認 |
| 物理スマートフォン | UNVERIFIED | 実施していない |
