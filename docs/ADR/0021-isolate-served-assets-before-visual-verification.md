# ADR 0021: visual verification前に候補サーバーの配信assetを分離する

## context

候補日カレンダーのsource CSSは、toolbarを三列、曜日と日付を七列にしていた。
しかし、`127.0.0.1:8081` のHTMLが参照した `/assets/main-dxhffd3c068dc3a2f3e.css` はHTTP 200を返しながら、`candidate-calendar-toolbar` と `candidate-calendar-grid` を含まなかった。

sourceの `assets/main.css` は41,917 bytes、配信fileは31,398 bytesだった。
同じ `target/dx` にはcalendar selectorを持つ別のhash付きfileも残っていた。
HTMLの新しい候補pickerと古いCSSが同時に表示されたため、markup自体の崩れに見えた。

二つの `dx serve` はportだけを分け、同じworktreeとbuild出力を共有していた。
Dioxus公式docsでは `asset!` がassetをbundleし、内容に対応するhash付きpathを生成する。
portの分離は、build artifactの分離を意味しない。

参考:

- [Dioxus 0.7: Assets](https://dioxuslabs.com/learn/0.7/essentials/ui/assets/)
- [Dioxus 0.7: Hot Reload](https://dioxuslabs.com/learn/0.7/essentials/ui/hotreload/)
- [MDN: minmax()](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/minmax)
- [WCAG 2.2: Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html)
- [WCAG 2.2: Target Size (Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)
- [WAI APG: Grid Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/)

## decision

- visual verificationは、source fileではなくbrowserが取得したHTML、Wasm、stylesheetを対象にする。
- 候補サーバーは専用portに加え、専用の `CARGO_TARGET_DIR` とDioxus `--session-cache-dir` を使う。別serverと同じ `target/dx` を共有しない。
- branchを切り替えた後は、既存serverのhot reloadを証拠にせず、分離した候補サーバーを新しく起動する。
- live HTMLからhash付きstylesheet URLを取り出し、そのURLが200、`text/css`、calendar固有selectorを持つことをbrowser操作より先に検査する。
- live DOMの説明文もsource固有の文言と照合し、SSRまたはWasmの世代不一致を検出する。
- toolbarの一行配置、七列、page scroll幅、target rectangleはcomputed styleとgeometryで判定する。source CSS assertionだけをvisual PASSに数えない。
- 日付buttonの数字は一つの短いlabelとして折り返さない。320pxではfieldset、calendar、grid gapの内側余白を詰め、24px四方以上のtargetを七列すべてで保つ。
- 320pxと1440pxのscreenshotを保存して目視する。pointerとkeyboardで月移動、時刻変更、日付toggleを操作する。
- calendarはnative buttonを維持する。ARIA `grid` はroving focusと矢印keyを実装するまで追加しない。
- 一連の確認では、calendarから匿名eventを作り、その共有画面から回答し、成功後の回答一覧まで確認する。
- 実ブラウザー検証で追加した匿名eventはignoredなlocal SQLiteだけへ保存し、識別子やcapabilityをreportとcommitへ残さない。

## rejected options

### source CSS assertionだけを増やす

今回のsourceには正しいselectorがあり、配信fileに欠けていた。
同じ失敗を検出できないため却下する。

### portだけを変えて二つのserverを動かす

HTTP listenerは分かれるが、build出力の書込先が同じままになる。
SSRとassetの世代を分離できないため却下する。

### `target/dx` の古いfileをすべて削除する

一時的には揃っても、二つのprocessが同じ出力先へ再び書けば再発する。
既存artifactの破壊的な削除を必要とせず、出力先を分ける。

### calendar CSSをinline styleへ移す

stylesheetの世代不一致をcalendarだけ回避しても、ほかのUIで同じ問題が残る。
CSSの保守性とcontent security policyの選択肢も狭めるため却下する。

### 見た目が整ったscreenshotだけを証拠にする

横overflow、grid track数、target幅は画像だけでは境界値を判定できない。
computed measurementとscreenshotの両方を残す。

## consequences

- 新しいmarkupと古いstylesheetを組み合わせた状態を、CSSの微調整前に検出できる。
- 候補serverの初回buildは遅くなり、分離したtarget directoryのdisk使用量が増える。
- live asset smoke testは配信世代を確認できるが、computed layoutを保証しない。Chromium measurementを別に実行する。
- screenshotは視覚的な配置を示せるが、keyboardとscreen readerの証拠にはならない。
- browser testがlocal SQLiteへdataを作る。`var/` のignoreと秘密情報scanをcommit前に確認する。
- 日付の数字を折り返さないため、さらに狭いviewportをpage側で縮小して収めることはしない。support下限は320 CSS pxのままとする。
