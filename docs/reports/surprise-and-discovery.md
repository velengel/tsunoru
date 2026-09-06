# Surprise & Discovery

## 限定公開版のブラウザー境界

2026-09-06、PR #13。Dioxus の `document::Title` は内部で eval を呼ぶため、自前の `document::eval` を除くだけでは厳しい CSP に適合しなかった。固定タイトルを静的 HTML に任せ、実際の画面操作で CSP 違反が出ないことを確認した。

release build でも CLI の debug symbols が既定で有効になり、wasm-opt が失敗して終了コード0のままになる試行があった。公開用は `--debug-symbols=false` を固定し、生成 Wasm の実行まで確かめた。また、古い hashed assets が出力先へ残るため、公開用の生成ディレクトリを再作成してからビルドする。

320px の viewport で縦スクロールバーが出ると有効幅は305pxだった。`body` と `html` の両方にある最小幅を限定版で解除し、カレンダーと集計の実測で横はみ出しを解消した。詳細と証拠は [report 0028](0028-staging-browser-app.md)。

UI の生成が済んでいても、続く Worker build の失敗で公開用ディレクトリに新旧の出力が混ざった。両方を一時 bundle にまとめて成功後に切り替え、失敗やシグナル中断でも直前の完成物を保つようにした。再現試験では失敗コードだけでなく、狙った途中工程へ到達した証拠も確認する。

## Batching needs a stopping condition

Grouping related fixes did not prevent repeated review passes from expanding this PR. The user's correction sets a two-round limit and makes scope, demonstrated impact and remaining risk part of the merge decision. Review convergence is no longer an unbounded completion requirement; ADR 0043 is authoritative.

Date: 2026-09-01

## Dioxus 0.7系の最新安定版

crates.ioで `dioxus` を検索すると、全系列の最新として0.8.0-alpha.1が先に表示された。
しかし、Cargoが0.7.3を解決した際には0.7.10を利用可能な更新として報告した。

アルファ版を避けることと、安定系列の最新を使うことは別の判断である。
`dioxus`、`dioxus-ssr`、Dioxus CLIを0.7.10へ揃え、`Cargo.lock` で解決結果を固定した。

## 公式テンプレートとCLIの版差

Dioxus CLI 0.7.10のBare-Bonesテンプレートは、`dioxus = "0.7.1"` とRust 2021 Editionを生成した。
CLIの版と生成されるdependency指定は自動では一致しない。

TSUNORUではテンプレートをそのままコピーせず、ADR 0003に従ってDioxus 0.7.10とRust 2024 Editionを明示した。
テンプレートからは `Dioxus.toml`、feature分離、asset linkの構成だけを参照した。

## Dioxus CLIの初回コスト

ローカル環境でDioxus CLI 0.7.10のインストールには4分46秒かかった。
最初のWeb buildでは、Dioxus CLIが `wasm-bindgen-cli` とesbuildを取得し、176個のコンパイル単位を処理して49.49秒で完了した。

RustでUIを記述しても、ブラウザーへ配信する過程からJavaScriptとJavaScript向けbuild toolが消えるわけではない。
利用者の初回準備を誤解させないように、READMEにはCLIの版固定と数分かかりうることを記録した。

## 同じコンポーネントを使う受け入れテスト

ブラウザー用の `App` コンポーネントは、テスト時に `dioxus-ssr` でHTML文字列へ描画できた。
この方法なら、ブラウザーを起動しなくても見出し、説明、landmark、accessible nameを同じRSXから検査できる。

CSS layoutとWebAssemblyの実行はSSRで保証できない。
その境界を埋めるため、Web build、開発サーバー、320pxと1440pxのChromium表示を別々に検証した。

## SQLx 0.9が要求するRust

SQLx 0.9.0はRust 1.94以上を要求し、作業開始時のstable 1.90ではコンパイルできなかった。
一世代前のSQLxへ下げなくてもstable 1.98へ更新できたため、TSUNORUの最低Rustバージョンを1.94へ上げた。

新しいlibraryを選ぶ判断と、古いtoolchainを維持する判断は分ける必要がある。
今回はRust frontendを学びながら現行技術へ追従する目的に合わせ、SQLxを下げるよりtoolchainを上げる方を選んだ。

## Fullstack featureの暗黙依存

Dioxusを `default-features = false` にすると、`fullstack` と `router` だけではcomponent macroと `launch` が利用できなかった。
単一crateのFullstack構成には、`lib` と `launch` も明示する必要があった。

featureを絞ること自体はWASMとserver dependencyを分けるために有効である。
ただし、名前から想像してfeatureを削るのではなく、client build、server build、all-featuresを別々に通す必要がある。

## SSR表示と操作可能になる時刻

FullstackのSSRはフォームをすぐ表示したが、debug WASMは47MBあり、初回のhydrateが終わる前はbuttonを押してもRustのevent handlerが動かなかった。
見えていることと触れることは同じ証拠ではない。

ブラウザーテストでは、formにDioxusのevent listenerを示す属性が付くまで待ってから操作した。
これにより、SSRの見た目ではなく、hydrate後の実際の作成経路を検証できた。

## commit後の再読込が作る失敗窓

最初の保存処理はSQLiteのtransactionをcommitした後、同じイベントを別queryで読み直して応答を作っていた。
そのqueryだけが失敗すると、画面は保存失敗と伝える一方、DBにはイベントが残り、再試行で重複する。

保存した公開データをtransaction内で組み立て、commit成功時にそのまま返す形へ変えた。
network応答を失う可能性までなくなるわけではないため、公開運用で厳密な再試行安全性が必要ならrequest IDによる冪等化を追加する。

## document全体の属性とHTTP status

RSXのcomponentだけでは、生成されるdocument rootに日本語の `lang` が付かなかった。
repository rootの `index.html` をDioxus templateとして管理し、`html lang=\"ja\"` を明示した。

また、動的routeが存在するだけでは、未知の公開イベントIDもHTTP 200になった。
Dioxusの `FullstackContext` でSSR responseを404へ確定し、public-by-link画面には `noindex, nofollow` を加えた。

## 型付きserver functionとHTTP status

Dioxusのserver functionが通常の `Result<T, E>` を返すだけでは、application errorに持たせたstatusがHTTP responseへ反映されなかった。
不正入力も内部失敗もHTTP 500に丸められ、画面側が再試行可否を区別できない。

公開APIのerrorを `ServerFnError` へ明示的に変換し、422、404、409、500をtransport境界で保持した。
一方、未知のenum文字列はhandlerへ届く前のrequest decodeで拒否され、Dioxus 0.7ではHTTP 500になる。
保存はされないが、型付き境界の外で起こるdecode errorまで同じstatus設計で制御できるとは限らない。

## migrationは最初のDB利用時に走る

SQLite poolを遅延初期化しているため、開発サーバーのrootがHTTP 200でもmigrationが成功した証拠にはならなかった。
公開eventの読み込みや保存など、DBを使う最初のrequestで初めてmigrationが適用される。

起動確認、DB経路の確認、`_sqlx_migrations` の確認を別の証拠として扱う。
また、適用済みmigrationを開発中に直した場合はchecksumが変わるため、作業用DBを削除せず退避し、fresh DBで再確認する必要がある。

## SSR testとDioxusのEventHandler

通常のDioxus `EventHandler` を公開componentのpropsに置くと、runtime外でpropsを組み立てるSSR受け入れtestがpanicした。
画面では正しいcallbackでも、test境界までDioxus runtimeへ依存していた。

ひとことcomponentには小さなcallback wrapperを渡し、親だけが回答capabilityを保持する形にした。
これにより、子componentのHTMLと入力状態をSSRで検査しつつ、生のcapabilityをprops、DOM、URL、browser storageへ出さない境界を保てた。

## SQLiteの文字数とNUL

SQLiteの `length(TEXT)` は最初のNULより後を数えない。
そのため、文字数上限のCHECKだけでは、直接DBへ書く経路で `短い文字列 + NUL + 長い文字列` を通せた。

server validationのNUL拒否に加え、DB CHECKへ `instr(text, char(0)) = 0` を置いた。
application validationとDB制約は同じ入力を守っていても、文字列関数の意味が同じとは限らない。

## 無視した旧frontendもtoolの探索対象になりうる

Gitで無視した `node_modules` がrepository内に残っていると、Dioxus CLIの探索対象になり、buildやserveが長時間進まないことがあった。
Gitの追跡境界とbuild toolの探索境界は一致しない。

置き換え済みReact基盤の依存物は `/private/tmp/tsunoru-superseded-react-20260902` へ退避した。
必要なら依存関係は再取得できるが、Rust/Dioxus基盤のrepository内には置かない。

## HTMLのescape表現と安全な描画境界

Dioxus SSRのtext nodeは、`<` や `&` を安全にescapeする。ただし、出力は `&lt;` のような名前付き文字参照ではなく、数値文字参照になる場合がある。
最初のtestは文字参照の綴りまで固定したため、表示を合わせる目的でescape済み文字列を `dangerous_inner_html` へ渡していた。

重要なのは表記ではなく、利用者のひとことがHTMLとして解釈されないことである。
testを同値な文字参照の双方へ対応させ、通常のtext nodeへ戻した。安全な標準境界を、test都合で強いAPIへ置き換えない。

## 認可の読み取りがSQLite snapshotを決める

主催者用サマリーでは、最初のSELECTをeventとcapability hashの照合にした。
SQLiteのDEFERRED transactionは最初の読み取りでsnapshotを確立するため、その後の回答数、候補集計、ひとことpreviewも同じ時点を見る。

file-backed WALと複数connectionのtestでは、認可後に別connectionから回答をcommitしても、進行中のサマリーは0件のままだった。新しいtransactionだけが1件を見た。
認可と集計を同じtransactionに置くことは、権限確認だけでなく、異なる時点の値を一つの画面へ混ぜない役割も持つ。

## private応答のcache制御は失敗経路にも必要

Dioxus 0.7.10のserver functionでは、`FullstackContext` へ追加したresponse headerがerror応答にも反映された。
入力検証より前に `Cache-Control: no-store` を設定し、成功時だけでなく422、404、500にも同じ方針を適用した。

private dataを返さなかったerrorでも、認可結果や再試行の情報を共有cacheへ残さない方が境界を説明しやすい。

## 同時再送testの一時停止

server全testの並列実行中、既存の `simultaneous_identical_retries_create_one_response` が一度だけ60秒を超えて停止した。
processを終了した後、同test単独とserver全testの一列実行はPASSし、以前の並列実行もPASSしている。

再現条件と原因は分かっていない。
同時実行を検証するtest自体に無期限待機を残さないため、再発した場合は非同期timeoutをtest harnessへ追加し、DB lock待ちとtask待ちを区別する。

## JOINが不変条件違反を検査前に消す

集計表のcellを回答tableとの `INNER JOIN` で読むと、通常データは効率よく取得できた。
しかし、未知のresponse IDを持つ破損cellはJOIN結果から消え、後段に未知IDを拒否するコードがあっても到達しなかった。

availabilityをevent IDで直接読み、Rust側で回答と候補の両方を照合する形へ変えた。
直接取得を全eventのcell scanにしないため、event-scoped indexも追加した。
不変条件を検査するコードだけでなく、SQLが異常な入力を検査地点まで保持するか確認する必要がある。

## 非同期requestの正しさには世代がある

集計表requestは `use_action` のresetでcancelできたが、既存のsummary更新と復旧は別々の `spawn` だった。
古い更新が復旧成功より後に失敗すると、消したはずの復旧formやerrorを再表示できた。

summary requestへ単調増加する世代を付け、完了時に最新世代だけがstateを更新するようにした。
Dioxusのeffect内で世代を通常readすると更新対象として購読し、自己再実行を招くため、世代判定には非reactiveな `peek()` を使った。

## CSS selectorのtestがrule順へ依存した

responsive testの小さなparserはselectorを前方一致で探す。
`.response-matrix-scroll-help` が `.response-matrix-scroll` より先にあると、別ruleを対象として失敗した。

production CSSの意味だけでなく、test helperの探索規則まで名前のprefixに影響された。
今回は局所scroll本体のruleを先に置いて解消したが、selector parserを拡張するときは完全なrule開始を照合する余地がある。

## 同時再送testは単体では速くsuite内で止まりうる

Story 5の全server testを一列実行した際、既存の回答再送testに加え、ひとこと再送testも別の試行で60秒以上停止した。
どちらも単体では0.02秒以内にPASSし、残りのsuiteも二件を除けばPASSした。

productionの冪等性は確認できたが、同期Barrierをasync testで使うharnessには無期限待機が残る。
後続Storyではtimeout付きの同期方法へ置き換え、suite schedulingとSQLite lockを区別できるようにする。

## migrationの前提はschema履歴全体で確認する

日程決定のcomposite foreign keyを設計した際、最初のmigrationだけを読み、candidate側に `(id, event_public_id)` のunique keyがないと判断した。
しかし、そのkeyはavailability responseのためにmigration 0002で既に追加されていた。

重複indexを作る前に全migrationと最終schemaを確認し、既存keyを名前非依存のtestで検証する形へ戻した。
段階的なschemaでは、初期DDLだけで現在の制約を判断できない。

## request世代とpending flagの所有権は別である

summary requestのepochは古いresponseが画面を上書きすることを防いだが、共有の `refreshing` booleanを誰が解除できるかまでは表していなかった。
復旧が停止中のrefreshを追い越すと、古いrequestが戻るまで日程決定をblockしたり、後から始めたrefreshのpending表示を古いrequestが消したりできた。

refreshにもowner epochを持たせ、supersede時は所有権ごと手放し、完了したrequestは自分がownerの場合だけpendingを解除するようにした。
「どの結果を採用するか」と「どの操作が進行中表示を所有するか」は、同じ非同期処理でも別々にモデル化する必要がある。

## Dioxusのserver function引数はwire上で一段包まれる

Rust関数が一つのstruct引数を取っていても、Dioxus 0.7の生成HTTP bodyはstructそのものではなく、引数名を持つ `{ "input": ... }` になる。
structだけを直接POSTすると、application validationへ届かずdecode errorの500になった。

typed clientは正しいbodyを自動生成するが、curlや外部clientで検証するときは生成関数の引数objectまで含める必要がある。
wire shapeが壊れたdecode errorは関数より前に返るため、関数内で設定する `no-store` の対象外になることもADR 0013へ記録した。

## projection追加は無関係なenumのsizeにも波及する

`OrganizerEventSummary` へ任意のdecisionを追加すると、同じ値をvariantに持つUI内部のload-state enumがClippyのlarge enum variant判定を超えた。
network projectionのfield追加が、通信やDBだけでなくclient stateのmemory layoutへも波及した。

大きい成功payloadだけを `Box` に置き、他の小さいstateを同じ大きさへ膨らませずに済ませた。
型安全なstate machineでも、variant間のsize差は保守時に確認する必要がある。

## IANA名を付けた固定offsetはそのtimezoneではない

一件の予定だけなら、選択日時のoffsetを `VTIMEZONE` へ埋めれば自己完結すると考えた。
しかし、`TZID:America/New_York` と名付けながら実際のDST transitionを持たない定義は、IANA timezoneとは別の固定zoneになる。
特に存在しないlocal timeと二重になるlocal timeで、calendar clientがTSUNORUと異なるinstantへ解釈しうる。

Story 7は保存したlocal startとIANA名をserverで一つのUTC instantへ解決し、`DTSTART` をUTCで渡す形へ改訂した。
短いtimezone定義を作ることと、既存timezoneを正しく表すことは同じではない。

## hidden属性はauthor CSSのdisplayに負ける

手動copy欄には `hidden` を付け、SSR testも属性を確認していた。
一方、同じ要素へ `.decided-event-manual-copy { display: grid; }` を指定したため、author CSSが初期非表示を上書きできた。

`[hidden] { display: none; }` を明示して、属性、Tab順、layoutの意図をCSS側でも保つようにした。
DOM文字列に属性があることと、computed layoutで見えないことは別の証拠である。

## path extractorの失敗はhandlerの方針を通らない

calendar handlerは不正UUIDを404と `no-store`、`nosniff` へ揃えていた。
しかし、percent decode後が不正UTF-8のpathはAxumの `Path<String>` がhandler前に400を返し、共通headerも付かなかった。

extractorを `Result` として受け、rejectionも同じ404 responseへ変換した。
route parameterを検証するだけでは、frameworkが検証より前に返すresponseまで統一できない。

## 文字数上限とUTF-8 byte長は別である

iCalendar folding testで日本語を長くしたところ、最初のfixtureはevent名の100文字上限自体を超えていた。
100文字以内でもUTF-8では300 octetsになるfixtureへ直すことで、domain上限を守ったまま75 octets foldingを検証できた。
文字数制約とwire formatのbyte制約は、同じ長い文字列でも別々に作る必要がある。

## Dioxus開発proxyでは外側のHostがbackendへ届かない

8081の実HTTPでsame-origin POSTを送っても、backendが見るHostはDioxus CLIの一時portだった。
一般的なforwarded headerもなく、Hostだけから外側のoriginとport別cookie名を復元できなかった。

Dioxus CLI自身がbackendへ渡す `DIOXUS_DEVSERVER_PORT` を確認し、backend Hostがloopbackである場合だけ外側portとして使った。
開発proxyの公開originは、慣例的なheader名を推測するより、実際のrequestと利用中CLIのsourceから確認する必要がある。

## password検証後のrate limitは計算も正解も止めない

最初のlogin limiterは、Argon2 verifyが失敗した後に回数を記録していた。
上限後も高価なverifyを毎回実行し、正しいpasswordなら制限を通り抜けるため、429を返す経路があっても試行制限にはなっていなかった。

試行をDB lookupとArgon2より前に原子的に予約し、上限後は正しいpasswordも同じ429へ止めた。
rate limitはerror文言ではなく、高価な処理と認証判定のどちらより前に置かれているかで評価する。

## 同じrouteへの遷移はprivate stateの破棄を保証しない

logout成功後に `/history` へreplaceすれば未login表示を読み直すと考えた。
しかし、同じrouteではcomponentのremountが保証されず、sessionを削除した後もlogin IDとevent名を持つlocal signalがDOMへ残り得た。

logout callbackで親のload stateをGuestへ明示的に変え、private projectionを直ちに破棄するようにした。
server上の認証失効と、共有画面からprivate情報が消えることは別々に検証する必要がある。

## HTML maxlengthとRustの文字数は同じ単位ではない

password domainはUnicode scalarを128文字まで許すが、HTMLの `maxlength` はUTF-16 code unitで数える。
絵文字はbrowser側で二単位になるため、同じ128を指定するとserverでは有効なpasswordを入力途中で拒否した。

password fieldのmaxlengthを外し、共通domainで文字数とUTF-8 octet数を検証した。
複数runtimeに同じ上限を書く場合は、数値だけでなく数える単位も一致させる。

## server function内のheaderはdecode前のerrorへ届かない

auth functionの冒頭で `no-store` を設定しても、routing、method、JSON decodeが先に失敗するとfunction codeは動かない。
private endpointほど失敗responseもcacheさせたくないため、function内だけでは境界が欠けた。

Axum middlewareでauthとaccount pathのresponseを後処理し、405を含むpre-handler errorにも `no-store` と `nosniff` を付けた。
frameworkがhandlerより前に返せるresponseは、handlerの共通処理とは別の層で揃える。

## sessionの事前確認は本体writeの原子性を強めない

cookie付きのevent作成と回答は、最初にsessionを確認してから、本体transaction内でも同じsessionを確認していた。
accountを安全に結び付ける意図だったが、SQLiteのwriter lockを二度取り、二つのtransaction間でsessionが失効するとcookie cleanupの判断も古くなった。

形が正しいsession hashはそのままrepositoryへ渡し、本体write transaction内で一度だけ解決するようにした。
repositoryは保存値と `Active`、`Inactive`、`NotPresented` のstatusを一緒に返し、serverはcommitしたstatusからcookieを消す。
認証情報の事前確認を増やすことと、認証結果とwriteを同じtransactionへ置くことは同じではない。

## 古いcookieの掃除が新しいloginを消すことがある

本体writeから `Inactive` statusを返せば、期限切れcookieを正確に消せると考えた。
DB上の判断は正しくても、古いwriteの削除responseと新しいloginの発行responseが並行すると、browserへ届く順番で新cookieまで消し得る。

event作成、回答、account履歴はsession cookieを発行も削除もしないようにした。
event作成と回答は形が正しいhashだけを本体transactionへ渡し、履歴は `Guest` または `Expired` の状態だけを返す。
stale cookieの上書きまたは削除はregister、login、logoutに限定する。
server内の状態を原子的にしても、複数HTTP responseがbrowser stateへ適用される順番までは原子的にならない。

## 通常propはeffectのdependencyにならない

private detailのrequestへgenerationを付けても、route parameterの `public_id` は通常のString propのままだった。
Dioxusの `use_effect` はeffect内でreadしたreactive値だけを購読するため、同じrouteでevent AからBへ遷移してもeffectが再実行されず、Aのprivate内容をBのURL上へ残せた。

`use_reactive` でpublic IDをdependencyへ変え、さらにpublic IDをkeyにしたroute contentをremountした。
古いrequestを採用しない世代判定と、route変更の最初のpaintで古いstateを持たないことは別の防御である。

## custom disclosureはmarkerとfocus clippingを引き受ける

`summary` をflex layoutへすると、browser既定のdisclosure markerを失った。
さらに親の `overflow: clip` がsummaryの外側へ描くfocus outlineを欠かせ、keyboardから開閉できても操作箇所を見失い得た。

custom markerを明示し、focus ringを親borderの外へ描けるようclipを外した。
native elementを使っていても、displayとoverflowを変更した時点で、既定のaffordanceが残るとは限らない。

## 同じ回答を保持するだけでは区別できない

repositoryは同じaccount、同じ表示名、同じpayloadの複数responseを正しく別rowとして返した。
しかし、UIで同じsummaryを二つ並べるだけでは、どちらが一件目か利用者にもaccessible nameにも現れなかった。

section内の安定順を使い「回答 1 / 2」のordinalを表示した。
deduplicateしないdata contractと、複数件を人が区別できるpresentation contractは別々に必要である。

## responseに必要なsecretもDebugには不要である

イベント作成成功は、browserが主催者権限を保存するため、生のcapabilityを一度だけJSONへ含める必要がある。
一方、同じ型の自動 `Debug` は、将来のlog、panic、test失敗へその生値を出せた。

`Serialize` は成功responseのため維持し、custom `Debug` だけを `[REDACTED]` にした。
秘密を返す必要がある型でも、wire、保存、debugの三つの表示境界は別々に決める必要がある。

## read planはwriteの予約ではない

continuation planはsessionのtouchを含むため、最初はseries全memberを読む間も `BEGIN IMMEDIATE` を保持した。
しかし、plan取得後からcreateまでの変更は防げず、匿名作成と回答が使うSQLite writerだけを長く待たせる。

session処理を短いwrite transactionでcommitし、ownerとtailはDEFERRED read snapshotで読むようにした。
保存時は別の `BEGIN IMMEDIATE` でsession、owner、tailを再検証する。
確認画面のsnapshotと、保存時に必要な排他は同じtransactionである必要がない。

## draftと最新planは別々に新しくなる

409後にplanを読み直すと、server上の最新tailと次回名候補は変わる。
ただし利用者が編集中のevent名、ひとこと、候補日時を自動で置き換えると、競合回復のために入力を失う。

最新候補をdraftとは別に表示し、明示操作でだけ名前へ反映した。
さらに一時的な401または500では候補表示を残し、新しい409で古くなった場合だけ破棄する。
server snapshotの鮮度、利用者draftの保持、復旧手掛かりの寿命は三つのstateとして扱う必要がある。

## account削除時のcascade順は実schemaで確かめる

series membershipはaccountとevent ownerの両方へ複合foreign keyを持つ。
account削除ではmembershipを消す一方、public eventは `organizer_account_id = NULL` として残すため、二つのdelete actionが干渉し得た。

membershipからaccountへ直接 `ON DELETE CASCADE` を持たせ、実migrationへaccount削除を行ったtestで、series関係だけが消え、eventとforeign key整合性が残ることを確認した。
複数のcascadeと `SET NULL` が交わる場合は、DDLを読むだけでなく最終状態を実行して確かめる必要がある。

## default値は入力済みという意味ではない

候補時刻へ `19:00` を入れたところ、日付が空でもpending candidateの片側だけが入力済みと判定された。
画面を開いてそのまま送ると、利用者が直接入力を始めていないのに「日付と時刻を両方入力してください」となった。

pending candidateの開始条件を日付側に置き、日付が空ならdefault時刻だけを無視した。
入力を助けるdefault値と、利用者が入力を開始したというdomain上の事実は分ける必要がある。

## 回答後の可視性は永続的な読取権限を増やさず作れる

みんなの回答を表示するために、最初は回答者向けの新しいGETやsecret保持が必要に見えた。
しかし、既存のresponse capabilityで保存を認可した直後なら、同じcapabilityがeventのresponseへ結び付くことをserver内で再確認し、その成功responseへsnapshotを同梱できる。

一覧専用endpoint、query parameter、cookie、localStorageを増やさず、「送った後」だけ一覧を返せた。
操作直後だけ必要なread modelは、永続的な読取経路よりwrite success payloadの方が小さい認可境界になる場合がある。

## ARIA roleは見た目の分類ではなく操作契約である

月間calendarは視覚的にはgridだが、`role="grid"` を付けると矢印key、Home、End、Page Up/Down、roving tabindexを期待させる。
今回必要なのは常設の複数選択であり、その一式を実装・実機検証する範囲ではなかった。

各日をnative buttonとしてTab、Enter、Spaceで操作できるようにし、`aria-pressed` で選択を伝えた。
ARIA roleはCSS layoutの説明ではなく、利用者へ約束するkeyboard modelとして選ぶ必要がある。

## 320pxの七列では縦横44pxを同時に要求できない

320pxからpage、form、fieldset、calendarのpaddingを差し引くと、七列の日付buttonは一列あたり約25pxになる。
各cellへ横幅44pxを要求すれば、calendarかpageを横overflowさせる。

高さ44pxを保ち、横幅はWCAG 2.2の24px以上、列は `minmax(0, 1fr)` とした。
target sizeは数値を一つ置くだけでなく、固定列数と最小viewportの実効content幅から逆算する必要がある。

## 透過previewはalpha channelの証拠にならない

最初のfavicon候補は外周にcheckerboardが見えたため、透明なPNGに見えた。
しかし、実fileはalpha channelを持たないRGB PNGであり、checkerboard自体が画素として描かれていた。

別の透過候補はRGBAだったが、深緑の内側にも意図しない透明部分が生じた。
16pxでの明瞭さを優先し、最終版は深緑を外周まで敷くRGB PNGとした。
透過はpreviewではなくfile metadataとpixelを検査する必要がある。

## assetのbyte差分はpixel差分とは限らない

Dioxusが配信したhash付きfaviconは、source PNGとbyte hashとfile sizeが異なった。
一方、両方をdecodeしたSSIMはRGB全channelで1.000000となり、pixelは一致していた。

sourceには色管理metadataがあり、配信版では除かれていた。
画像assetの同一性は、byte比較だけでなくdimension、channel、decode後のpixelを分けて確認する必要がある。

## `decision`の長さはADR境界の崩れを示す

Date: 2026-09-02

ADR 0012の`decision`は、確定結果の不変性に加えて、transaction、schema、endpoint、error、projection、画面挙動を一つの節で決めていた。
各項目を残したまま文章だけ短くしても、一つのADRへ複数の判断を入れた構造は変わらない。

今後は採用した判断を一行で述べる。
一行に収まらない独立した判断が現れたら、詳しい箇条書きを足す前にADRを分ける。

## worktreeの作成先を変えてもGit objectの置き場は変わらない

二つのworktree作成が`/private/tmp`で遅れたため、最初は同時実行か作成先の問題に見えた。
しかしlinked worktreeが分けるのはHEADとindexなどであり、objectはmain worktreeのGit directoryを共有する。

TSUNORUの共通objectはiCloud Drive上にあり、926ファイル中616ファイルが`dataless`だった。
465 byteのobject一件でも、実際に読むと約30秒待った。
worktreeのpathだけでなく、共通Git directoryがローカルへ実体化されているかを作成前に見る必要がある。

## `locked initializing`は失敗後の残骸とは限らない

Gitはworktreeの準備中、管理directoryをpruneから守るため`locked` fileへ`initializing`と書く。
準備が終わればGit自身が削除する。

短いcommand yieldの直後にこの表示を見ても、checkoutが失敗したとは限らない。
実行session ID、Git process、indexとfileの更新を確認し、同じsessionがexit codeを返すまで待つ必要がある。

## background testはsandboxで優先度変更に失敗し得る

zshはbackground jobを自動的にniceする設定を持つ。
managed sandboxではその優先度変更が拒否され、同時作成testがlock取得前に止まった。

testだけで`BGNICE`を無効にすると、production scriptのprocess優先度を変えずに同時実行を再現できた。

## folderがdownload済みでも子孫は保持済みとは限らない

File ProviderはTSUNORU folder自体を`isDownloaded=1`と報告した。
しかし同時に`isRecursivelyDownloaded=0`、`isKeepDownloaded=0`であり、再計測した`dataless` fileは短時間で増えた。

一fileを読んで実体化することと、repositoryの子孫を今後もMacへ保持することは別である。
Gitの安定動作にはfolder単位の「ダウンロードを保持」と、その後の再帰的な0件確認が必要になる。

## Keep Downloadedは完了通知ではない

Finderで「ダウンロードを保持」を指定すると、File Providerは`isKeepDownloaded=1`へ変わり、多くのplaceholderを実体化した。
しかし残りは`isRecursivelyDownloaded=0`のまま止まり、内容を読むsessionも進まなかった。

保持方針と現在のlocal byteは別のstateである。
設定済みという表示だけでGitを再開せず、metadata preflightが0件になることを完了条件にする必要がある。

さらに今回、残数は121/24件まで減った後、141/45件へ再増加した。
現在のPC状態ではKeep Downloadedだけを恒久的なGit安定性の証明にできず、repository本体のFile Provider外移行が必要になる。

## Finderの終了ではなく作業差分を閉じる

調査で`open -R`を繰り返した結果、同じTSUNORU folderのwindowが3枚増えていた。
一方で、Finderには利用者が元から開いていたDownloads windowとMP3の情報windowもあった。

Finderを終了する、または全windowを閉じるcleanupでは、利用者の作業まで失われる。
GUI調査は開始前のwindow ID、target、bounds、情報・詳細panelを記録し、終了後に増分だけを閉じる必要がある。

## 別repositoryのworktreeも同じCloudDocs queueを使う

別sessionのworktreeは異なるrepositoryやbranchなら、GitのHEADとindexを直接共有しない。
しかしFile Provider配下へ作ったcheckout fileは、repositoryに関係なくPC全体のCloudDocs同期queueへ入る。

今回のqueueには別repositoryの`.codex/worktree`、Git object、source fileが大量に並び、約49分間`needs-sync-up`のままだった。
共通Git directoryをローカルに保つだけでなく、linked worktreeの作成先もFile Provider domain外へ出す必要がある。

## 履歴なしsnapshotはtreeの同一性をhashで証明できる

既存履歴を捨てる移行では、file copyだけを見ると、公開treeが元の検証済みtreeと同じか判断しにくい。
今回、`git archive`でtracked fileだけを展開し、root commit前の`git write-tree`と移行元commitのtree objectを比較したところ、両方が`01bc701bccf78a2714e2f7b89ac9d6ade47e202a`で一致した。

commit履歴を引き継がなくても、file内容、path、実行bitを含むtree objectは照合できる。
履歴なし移行では、公開前のsecret scanとtree hash比較を別の証拠として残す必要がある。

## 空repositoryへの最初のmain pushは通常のPR gateを通せない

公開用repositoryにはdefault branchがなく、PRのbaseにできるbranchもなかった。
一方、Codexの実行環境はmainへの直接pushを実行前に拒否したため、利用者が検証済みstaging repositoryから最初の一回だけpushした。

GitHubでdefault branchがmainになり、remote HEADがroot commit`351bddffddd873d9b95ef55d7a7cad17b86fe8b8`と一致することを再確認した。
空repositoryの初期化だけは、通常のfeature branchとPRによる更新とは別の移行境界になる。

## remote commitから作るlocal branchは追跡設定を別に確認する

`create-feature-worktree.zsh`へremote branchをstart pointとして渡すと、branchとworktreeは期待したcommitで作られたが、upstreamは自動設定されなかった。
作成後のbranch、HEAD、clean status、lock不在だけでは、次回push先まで決まった証拠にならない。

移行後に`git branch --set-upstream-to`を実行し、`git branch -vv`で同名remote branchの追跡を確認した。
既存remote branchをrepository内worktreeへ復元するときは、upstreamを独立した検証項目にする。

## commitしなかったstage内容もlocal Git objectに残る

移行reportを最初にstageした後、個人absolute pathを`$HOME`表記へ直して再stageした。
最終fileと公開commitには個人absolute pathがなかったが、最初にstageした内容は到達不能blobとして新しい正本のobject storeに残っていた。

`git fsck --unreachable --no-reflogs`で他の到達不能objectがないことを確認し、対象blobをpruneした。
公開前scanはcommitとremoteへの混入を防ぐが、local object storeまで消すものではない。
秘密情報や公開しない個人情報を一度stageした場合は、到達可能履歴とは別にunreachable objectも確認する必要がある。

## WAL modeのDB本体だけではreadonly openできない場合がある

停止中のSQLiteはWALが0 byteだったため、DB本体だけを新配置へ複製した。
DB checksumは一致したが、SQLite CLIの`-readonly` openは、WALとSHMがない状態でexit code 14になった。

`immutable=1`のreadonly URIではintegrity checkが`ok`だった。
書込可能な通常openも`ok`となり、0 byteのWALと32 KiBのSHMを再生成したが、DB本体のchecksumは変わらなかった。
WAL modeのlocal stateをDB本体だけで復元するときは、破損と判断する前にreadonly接続方式と一時file再生成を分けて確認する必要がある。

## 削除中にもsystem metadataは作られる

旧iCloud rootの`rm -rf`は約29 GiBを削除した後、空にしたはずの複数directoryを`Directory not empty`として残した。
残っていたのは、削除開始後の時刻を持つ`.DS_Store`が5file、44 KiBだけだった。

process一覧と`lsof`にはCargo、Dioxus、Gitによる旧pathへの書込みがなかった。
どのmacOS componentが作ったかは特定できないため、FinderまたはFile Providerのどちらかへ原因を限定していない。

一回目の削除失敗だけを再実行の根拠にせず、残存fileとprocessを読み直した。
source、Git履歴、利用者dataが残っていないことを確認した後、同じrootを削除してpath不在を確認した。

## Post-migration runtime checks can own disposable data

On 2026-09-05, automatic review rejected an ad-hoc temporary lifecycle script and recommended a repository-scoped script.
The reviewed replacement, `scripts/verify-runtime.py`, owns a loopback server and a disposable SQLite backup, then checks the original SQL dump and database hash.
It passed review and the full anonymous HTTP lifecycle without broadening global permissions.
This session's approval does not establish a general allow rule.

A plain query-string call to `get_public_event` returned a missing-field HTTP 500.
The installed Dioxus 0.7.10 JSON extractor reads the argument body, including for GET; the matching request returned HTTP 200 for the migrated data.
See [the runtime report](0018-post-migration-runtime-verification.md) and [ADR 0026](../ADR/0026-isolate-runtime-verification.md).

## Calendar geometry and hydration require separate checks

On 2026-09-05, the pre-repair 320px page had seven grid tracks and no page overflow, yet two-digit dates wrapped to 32px-high labels.
The preserved narrow-layout repair kept labels at 16px and widened targets from about 24.98px to 28.05px.
A seven-column assertion alone would have missed the visible defect.

During browser verification, SSR response inputs existed before Dioxus attached handlers, so fast automated input was lost during hydration.
The runner now waits for the input's `data-dioxus-id`, as assigned by the installed Dioxus interpreter, before entering the response.
This establishes the hydrated path, not a guarantee about user input made before hydration.
The radio is operated through its visible label; force-clicking its visually hidden input is unnecessary.

See [Story 0015's browser report](0019-calendar-browser-verification.md).

## PR readiness includes asynchronous review completion

PR #6's ready flag triggered a Codex review that completed after the initial delivery message.
The review identified ADR 0021 as a newly introduced record with several independent decisions; its lower number had been mistaken for an accepted-history exemption.
The fix separates those decisions, and AGENTS.md now requires explicit final-head review completion and disposition of every finding before a PR-ready claim.
See [Story 0022](../story/0022-converge-codex-pr-review.md) and [ADR 0030](../ADR/0030-require-codex-review-convergence-for-pr-ready.md).


## Verifier signal ownership must include Playwright

The second Codex review exposed a termination path missing from the browser verifier.
A signal delivered to Node alone did not complete cleanup; Playwright also installs signal handlers by default.
Explicit verifier handlers now own SIGINT/SIGTERM and call the same idempotent cleanup as normal exit, with tests checking child-process and temporary-data removal.
See [the review dispositions](0019-calendar-browser-verification.md#termination-signal-follow-up).


## Review convergence extends across sibling verifiers

After the browser verifier's signal cleanup was fixed, Codex found the equivalent Python verifier gap.
A dedicated fixture reproduced the live server left behind by direct SIGTERM, and both signal paths now unwind Python's existing cleanup.
A cross-check of sibling verification tools is useful when a lifecycle defect is found in one tool.


## Signal cleanup must cover resource acquisition

A later review caught an earlier lifecycle gap: registering cleanup after initialization leaves startup resources unprotected.
The expanded browser regression now stops the verifier at four acquisition phases with both termination signals.
Cleanup is installed first and can await pending resource acquisition before removing it.


## A synchronous subprocess can block registered signal handlers

The expanded startup cleanup still left a synchronous asset check that could block Node's event loop during a stall.
The checker is now an owned asynchronous process group, and the signal regression includes a deliberately stalled checker.
Cleanup waits for both browser and checker cleanup even if one reports a failure.

## Review guidance and review triggers have different configuration locations

Official Codex documentation explicitly supports a Code Review Rules section in applicable AGENTS.md files, while hosted automatic-review toggles are managed in Codex settings.
The existing development workflow therefore needed a concise review-specific section, not a new configuration format.
See [the source-backed configuration record](0020-codex-review-configuration.md).

## A launched-browser checkpoint does not cover pending launch

The new repository review rule exposed an untested interval before Chromium launch resolves.
A never-settling launch promise and an actual unresponsive process now have dedicated signal regressions.
A bounded cleanup wait allows the owned server and disposable database to be reclaimed even when browser startup fails; the runner reports failure rather than claiming successful cleanup.

## A subprocess can exist before its cleanup handle is assigned

Python signal handlers can interrupt a Popen constructor after it starts the server.
Deferring the termination action until assignment closes this ownership gap without passing blocked signal masks into the child.
The regression now observes both pre-publication and ready-state termination of a real server.

## Process names are a poor ownership protocol

A macOS ps comm path and lsof lookup made the new launch regression depend on host-specific tools.
The verifier now publishes its PID and disposable path directly, while numeric PID/PPID inspection is used only to check descendants.
A fixed constrained-PATH fixture verifies that process-name and lsof queries are no longer required.

## Matching assets do not identify the writable database

A real TSUNORU server with the same bundle but another disposable database passed the old browser checks and received two event writes.
The equivalent HTTP fixture received one, so the batch review extended the fix across both verifiers before another external review.
Private database markers now establish the intended target before HTTP test mutations.

## Read-only SQLite can still affect source sidecars

A WAL fixture without SHM reproduced source-file changes during mode=ro inspection.
SQL inspection now runs only against a byte-stable disposable snapshot, including committed WAL data, while the complete source file set is compared without a SQLite connection.

## Review judgments can guide a broader local pass

The user requested a dedicated judgment log and commit-linked completion replies.
The current batch groups related isolation findings and cross-checks both verifiers, rather than triggering another review after each fix.
The log retains earlier reasoning and links while allowing reassessment when current evidence changes.

## TemporaryDirectory publishes cleanup after creating the path

Python's TemporaryDirectory can be interrupted after mkdtemp creates a path but before its finalizer is installed.
Deferring termination through ExitStack registration covers that interval and keeps later snapshot and database work inside an already-owned cleanup scope.

## A cancellation-safe verifier still needs a cancellation-safe caller

Terminating the outer regression driver bypassed Python's default cleanup even though the inner verifier handled the same signal. The failure reproduced a surviving seed server. Shared ownership scopes now cover all related drivers, including the new outer regression runner, and stop children before removing their working directories. Ten outer-driver signal cases complement the inner startup-phase regressions.

## Some cleanup risks disappear when the resource is unnecessary

The real shell asset checker still had a mktemp-to-trap gap that a stalled replacement fixture could not reveal. Its HTML and CSS only need comparison, so they now stay in memory. Removing those temporary files closes the interruption gap without another layer of ownership and signal handling.

## A connection object may silently become a different connection

A valid identity response did not prevent later writes to a replacement listener. Even one Python HTTPConnection object could reconnect automatically. Both verification clients now bind identity and mutation to a single physical connection and refuse reconnection; deterministic handover tests show zero replacement writes after the fix.

## Leader completion does not mean process-group completion

An owned fixture child survived Harness.stop after its parent exited. Cleanup now tracks the dedicated group until no members remain, including a TERM-resistant member, and retires the cleanup registration after completion. Python reaps its leader before group probes and only treats a permission failure as completion when the group is confirmed absent.
## 2026-09-05 公開計画

- Cloudflareは新規構成でWorkers Static Assetsを推奨するが、Rust対応は現行Dioxus/SQLxバイナリの互換性を意味しない。Containersもローカルdiskは非永続であり、配置先だけ替える案には保存設計が残る。
- 主催権限はlocalStorageにあるため、DB移転だけではoriginや端末を越えて引き継がれない。account履歴も主催権限の代わりにならない。
- backup復元は失効済みsessionや削除を巻き戻し得る。復元後の失効と削除再適用を公開条件へ追加した。
- 一次資料とコードの根拠、採用しない案、検証1往復の結果は[公開計画](0021-publication-plan.md)と[判断ログ](../review-judge-logs.md)を正とする。
## 2026-09-06 staging の認証と再送

- #9 の upsert は同じ回答の再送で行を増やさないが、回答者間の所有権も全候補の完了も保証しなかった。個別 capability と payload の一致を別々の条件にした。
- D1 batch が原子的でも、書込み条件に権限がなければ他人のデータを原子的に書き換えてしまう。条件付き INSERT と同時送信試験で両方を確認した。
- Miniflare 5 の options は既存の検証コードと互換ではなかった。公式 converter と明示した JS/Wasm modules で新規 checkout から再現できた。
- 詳細と証拠は [report 0027](0027-staging-authorization.md)、判断の再評価は [R040–R044](../review-judge-logs.md) を参照。

## 2026-09-05 Cloudflare小実験

Dioxus 0.7.10の最小server構成もmioのWasm非対応で失敗した一方、browser構成はcheckが通った。
既存domainをWasmで使う場合はserver featureを維持しないとtimezone検証が弱くなる。
D1 batchで更新0件でも先行INSERTがcommitされる反例を実測した。
根拠と限界は[小実験結果](0023-cloudflare-runtime-spike.md)を正とする。
