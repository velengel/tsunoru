# Story 0007 検証記録

Date: 2026-09-02

対象commit: `db34082 test: specify organizer response matrix`、`bb87a29 test: expose matrix corruption and stale summary races`、`4cc98a2 test: require event-scoped matrix lookup`、`b320918 feat: add organizer response matrix`

## 結論

Product Story 5の主催者用集計表は実装済みである。
主催者が回答サマリーから明示的に開いた場合だけprivate POSTを送り、全回答者と全候補の○、△、×をnative tableで表示する。

自動test、file-backed WAL snapshot、Fullstack build、候補版HTTPはPASSした。
実ブラウザーで320px、desktop、keyboard、VoiceOverを操作する検証は未実施のため、Storyの状態はin progressとして残す。

## test-firstの証拠

利用者向けHTML、repository、server function、WAL snapshot、responsive contractは `db34082` で実装より先にcommitした。

- domain、UI、storage、serverの未実装symbolにより対象testはcompile errorでREDになった。
- responsive testは `.response-matrix-section` がないためREDになった。
- 独立レビュー後、未知response IDの余剰cellと古いsummary requestの後着競合を `bb87a29` で先にREDにした。
- 完全性検査のためにevent IDで直接cellを読むとquery planが全table scanになった。event-scoped indexの利用を `4cc98a2` で先にREDにした。

## 実装境界

domain projectionはevent名、timezone、候補日時vector、回答者名と三値vectorだけを持つ。
候補ID、回答ID、capability、hash、コメント、件数、判断補助、日程決定を含めない。

repositoryはDEFERRED read transactionの最初のSELECTでevent IDとorganizer capability hashを照合し、同じsnapshotから次を読む。

1. `position ASC` の全候補。
2. 内部response ID昇順の全回答。
3. event IDに属すると記録された全availability cell。

cell queryの返却順には依存せず、内部IDから `R × C` の領域へ配置する。
欠損、重複、未知response、未知candidate、不正availability、積のoverflowがあれば部分表を返さない。
回答0件は全候補と空の回答vectorを持つ正常結果である。

主催者用endpointは `POST /api/organizer/events/matrix` である。
64文字の小文字16進数を検証し、SHA-256 hashへ変換した後に生のcapabilityを破棄する。
不在event、誤った値、別eventの値は同じ404にし、成功と全errorへ `Cache-Control: no-store` を付ける。

## UIとresponsive

- 初期表示とSSRではmatrixを取得しない。明示buttonを初めて開いた時だけ `use_action` を呼ぶ。
- buttonは `aria-expanded` と `aria-controls` を持つ。
- loading、失敗、再試行、0件をsummary本体と分け、通常失敗では表示中のsummaryを残す。
- capability不在または拒否ではmatrixを破棄して閉じ、既存の復旧formを表示する。
- 復旧またはsummary更新の成功時は進行中action、payload、DOMを破棄する。
- 古いsummary requestは世代tokenで後着結果を捨てる。effectが世代更新を購読しないよう、判定は `peek()` で行う。
- tableはcaption、列の `scope="col"`、行の `scope="row"` を使い、記号と「行ける」「条件次第」「難しい」を同じcellで伝える。
- pageではなく名前付きのtable領域だけをkeyboard focus可能な横scrollにする。固定するのは先頭の回答者列だけである。

## 自動検証

統合後に次を確認した。

```text
cargo test
  PASS: default 50 tests

cargo test --features server
cargo test --no-default-features --features server
  PASS: 各93 testsを分割実行

cargo clippy --all-targets --all-features -- -D warnings
  PASS

cargo fmt --check
  PASS

dx build --web
  PASS: client build and server build

git diff --check
  PASS
```

Story 5固有では、UIとprojectionの4件、repositoryの6件、server境界の3件、file-backed WAL snapshotの1件、responsive contractの該当testがPASSした。

server suiteへ含めると、既存の `simultaneous_identical_retries_create_one_response` と `simultaneous_identical_comments_create_one_value` が、それぞれ別の試行で60秒以上停止した。
processを終了し、二件を単体で実行すると各1件がPASSした。二件を除く残り91件も、default featureありとserverだけの両構成でPASSした。
Story 5のtest失敗ではないが、suiteとして一回のcommandで完走した証拠とは区別する。

## HTTPと稼働状態

- 候補版 `127.0.0.1:8081` のrootは200。
- 検証済み版 `127.0.0.1:8082` のrootも200を保った。
- 架空の形式不正matrix requestは422と `Cache-Control: no-store` を返した。
- 形式が正しく存在しないeventとcapabilityの組は404と `Cache-Control: no-store` を返した。
- 成功応答は隔離SQLiteを使うserver testで検証した。実在する秘密値をHTTP確認へ流していない。

## SQLite snapshotとquery plan

productionと同じfile-backed WALのtestは、認可でreader snapshotを確立した後、別connectionから回答をcommitした。
進行中のreaderは回答0件の完全なmatrixを返し、新しいtransactionだけが回答1件を返した。

availabilityを回答とのINNER JOINで読む初期案は、未知response IDを持つ破損cellを検査前に消した。
event IDで直接取得してRust側で両IDを照合するよう修正し、破損fixtureが `DataInvariantViolation` になるtestを追加した。
`EXPLAIN QUERY PLAN` は `response_availabilities_event_public_id_idx` を使い、全eventのcellをscanしない。

## 機密情報

- raw organizer capabilityはlocalStorage読取helperとPOST requestの局所値だけにある。
- matrix用Signal、projection、props、DOM、URL、logへcapabilityを保持しない。
- server responseのJSONは、fixtureのraw capabilityだけでなく実際のSHA-256 digestも含まない。
- private errorのDebug、Display、logはDB error、回答者名、コメント、capabilityを出さない。
- SQLite、WAL、SHM、退避DBは `var/` にあり、Git対象外である。
- 既知の秘密ファイル名を対象にした検索は一致しなかった。

## 独立レビュー

domain、storageを担当外のagentが、server、UI、CSSを担当外のagentがread-onlyでクロスレビューした。
初回はP0なし、P1が二件、P2が複数あった。

P1の未知response cell消失とsummary request後着競合は、RED testを追加して解消し、担当外agentが再確認した。
P2の実hash非露出test、復旧キー説明とvalidatorの不一致、event-scoped query planも解消した。

残るP2は、動的なasync state遷移とHTTP headerの一部がsource契約または分割testであること、実ブラウザー証拠がないこと、test harnessの同時再送testに無期限待機があることである。

## 実ブラウザー

次はUNVERIFIEDである。

- 320pxとdesktopでのpage overflow、table局所scroll、sticky回答者列、長い名前、最大20候補。
- keyboardだけで集計表を開閉し、横scroll、失敗、再試行、復旧、summary更新を操作できること。
- VoiceOverがcaption、行見出し、列見出し、三値の意味を読み上げること。
- 回答者contextからprivate matrixを読めず、DOM、URL、browser storage、consoleへ秘密値が出ないことの動的確認。

利用可能なin-app browser clientは起動できず、外部Playwright scriptは追加承認なしに実行していない。

## 一次情報

- [WAI: Tables with Two Headers](https://www.w3.org/WAI/tutorials/tables/two-headers/): 行と列の見出しをnative tableで関連付ける根拠。
- [WAI: Caption & Summary](https://www.w3.org/WAI/tutorials/tables/caption-summary/): 表の用途と操作説明を分ける根拠。
- [WCAG 2.2 Understanding: Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html): 二次元tableの例外をpage全体へ広げず、表だけを局所scrollにする根拠。
- [Dioxus 0.7.10: use_action](https://docs.rs/dioxus-hooks/0.7.10/dioxus_hooks/fn.use_action.html): 明示呼出しと進行中taskのcancelに使う根拠。
- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html) と [Isolation](https://www.sqlite.org/isolation.html): 一つのread snapshotで認可と全cellを読む根拠。
- [SQLite: ORDER BY](https://www.sqlite.org/lang_select.html#orderby): 候補と回答の決定的な順序を明示する根拠。

## 証拠の境界

| 層 | 状態 | 証拠 |
| --- | --- | --- |
| Git | PASS | `db34082`、`bb87a29`、`4cc98a2`、`b320918` |
| Rust test / lint / format | PASS | default 50件、server各93件を分割、Clippy、format |
| Fullstack build | PASS | client、server成果物 |
| local HTTP | PASS | 候補版8081と安定版8082のroot 200、private 422/404のno-store |
| SQLite | PASS | 認可、全cell、不変条件、WAL snapshot、event-scoped index |
| Chromium 320px / desktop | UNVERIFIED | browser client起動不良、外部script未承認 |
| VoiceOver / 物理スマートフォン | UNVERIFIED | 実施していない |
