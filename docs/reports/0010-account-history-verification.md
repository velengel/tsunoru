# Story 0010 検証記録

Date: 2026-09-02

対象commit: `2c2d5a0 docs: define optional account history`、`34bf94d docs: decide optional account history`、`1c60eed test: expose optional account history`、`0c075f2 feat: persist account-linked history`、`de59f29 feat: expose account session endpoints`、`ac2a2d7 feat: add optional account history screens`、`72de22a test: expose account lifecycle review gaps`、`412e84c docs: tighten account lifecycle decisions`、`b7cd0f5 fix: harden account lifecycle transitions`、`349933f docs: keep optional session resolution inside writes`、`1f6fd3b test: expose duplicate session resolution`、`49d7c8f fix: resolve account session inside optional writes`、`66bd015 docs: isolate auth cookie changes from public writes`、`5a2d872 test: expose public-write cookie race`、`d6596b5 fix: keep public writes from mutating auth cookies`、`6ddad02 docs: keep history responses from mutating auth cookies`、`087cc70 test: expose history cookie race`、`fe9a034 fix: keep history from clearing newer login cookies`

## 結論

Product Story 8の任意accountと主催・参加履歴は実装済みである。
利用者はloginせず従来のevent作成と回答を続けられ、履歴を使いたい場合だけaccountを作成する。
login中に新しく主催または回答したeventは、追加入力なしで二つの履歴へ結び付く。

password hash、server session、pre-hash試行制限、same-origin境界、SQLite transaction、最小履歴projection、登録・login・logout UI、自動test、Clippy、format、Fullstack build、候補版HTTPはPASSした。
320pxとdesktopでの実ブラウザー操作は未実施のため、Storyはin progressとして残す。

## test-firstの証拠

domain、password、session、repository、server function、UI、responsive contractは `1c60eed` でproduction実装より先にcommitした。
未実装module、type、function、route、componentにより対象testは期待したcompile errorでREDになった。

実装中にDioxus開発proxyが外側のHostをbackendへ転送しないことが分かり、port別cookieとoriginを実HTTPで再現した。
`DIOXUS_DEVSERVER_PORT` を使う回帰testを先にREDにしてから、loopback backendでだけ外側portを復元するよう修正した。

独立レビュー後は `72de22a` で、旧session削除失敗、上限後の正しいpassword、期限切れcleanup、logout後のprivate DOM、429案内、Unicode password、responsive selectorを先にREDにした。
ADRを `412e84c` で更新した後、`b7cd0f5` で一括修正した。

再レビューで、event作成と回答がsessionを事前transactionと本体transactionで二度解決していることが分かった。
`349933f` で本体transactionからtyped statusを返す方針をADRへ追加し、`1f6fd3b` で二重解決と期限切れstatusをREDにした後、`49d7c8f` で一本化した。

最終レビューでは、古いpublic writeのcookie削除responseと新しいloginのcookie発行responseが並行すると、到着順で新sessionを消し得ることが分かった。
`66bd015` でpublic writeがauth cookieを変更しない方針へ改訂し、`5a2d872` でsource contractをREDにしてから、`d6596b5` でcookie変更をauthとhistory境界へ限定した。

その後、別tabの古いhistory GETにも同じraceが残ると分かった。
`6ddad02` でcookie変更をregister、login、logoutだけへ限定し、`087cc70` で三つのdata endpoint本体を検査するtestをREDにしてから、`fe9a034` でhistoryを状態返却だけにした。

## passwordとsession

login IDは前後の空白を除いてASCII小文字へ正規化し、3文字以上32文字以下の限定した文字だけを受け付ける。
passwordはtrimせず、15文字以上128文字以下、UTF-8で512 octets以下とする。

passwordはArgon2id version 19、19,456 KiB、iteration 2、parallelism 1、32-byte output、毎回異なるsaltのPHC stringとして保存する。
hashとverifyはSQLite transactionの外で最大4件のblocking taskとして動かす。
存在しないlogin IDもdummy hashでverifyし、不在とpassword誤りを同じ公開errorへ揃える。

login attemptはDB lookupとArgon2より前に予約する。
正規化login IDのdigestごとに15分5回、最大4,096 IDへ制限し、上限後は正しいpasswordも429にする。
account作成もhash前にprocess全体で15分100回へ制限する。
どちらも単一processの初期防御であり、公開前にはTLS ingress側の信頼できる送信元を使う分散rate limitが必要である。

session tokenは64文字のrandomな16進文字列をbrowserへ一度だけ渡し、SQLiteにはSHA-256 digestだけを保存する。
sessionは7日idle、30日absoluteで期限切れにし、一時間ごとにだけlast-seenを更新する。
logoutはserver rowを削除してcookieを期限切れにするが、匿名主催者capabilityを持つ `localStorage` は削除しない。
同じbrowserがaccount作成またはloginをし直すときは、旧sessionのDELETEと新sessionのINSERTを同じtransactionで行う。
DELETEをtriggerで失敗させたtestでは、account作成と新sessionの双方がrollbackし、旧sessionだけが残った。

production cookieは `__Host-tsunoru-session`、localは `tsunoru-session-local-{port}` とする。
どちらもHttpOnly、SameSite=Lax、Path=/を持ち、productionだけSecureを必須にする。
production originは `TSUNORU_PUBLIC_ORIGIN` の明示値だけを信頼し、未設定時はloopback HTTPだけを許す。
この設定はTLSを提供しないため、TLS終端、HTTP redirect、HSTS、backend遮断のdeployment evidenceがない状態をinternet公開可能とは扱わない。

## CSRFと公開境界

cookie認証を追加すると、従来の匿名POSTもlogin中は履歴へ関連付くwriteになる。
そのためunsafeな `/api/` requestは、設定済みoriginと完全一致する `Origin`、または `Origin` がない場合の同一origin `Referer` を要求する。
不一致、`null`、両方なしはserver functionのdecode前に403へする。

認証、履歴、CSRF拒否には `Cache-Control: no-store` と `X-Content-Type-Options: nosniff` を付ける。
authとaccount pathはAxum middlewareでもresponseを後処理するため、server functionへ届かないmethod・decode errorも同じheaderを持つ。
account ID、password、生session、内部hashをrequest URL、HTML、公開projectionへ含めない。

account sessionは履歴の関連付けにだけ使い、主催者capabilityと回答capabilityの権限を置き換えない。
履歴項目は既存のpublic eventへだけlinkし、主催者用summaryへ直接linkしない。

## SQLiteと履歴

migration 0006は `accounts` と `account_sessions` を追加し、既存の `events` と `responses` へnullable account foreign keyを加える。
account削除ではpublic eventと回答を残して関連だけを `SET NULL` にし、sessionは `CASCADE` する。

login中のevent作成はevent、候補、主催accountを一transactionで保存する。
login中の新規回答はresponse、候補ごとの都合、回答accountを一transactionで保存する。
cookieなし、壊れたcookie、失効・期限切れcookieでは匿名writeを続ける。

保存済み匿名回答をlogin後に同じcapabilityで再送しても、既存rowをaccountへ変更しない。
Story 8より前のrow、login前のrow、似た名前を持つrowも遡ってclaimしない。

event作成と回答では、形が正しいcookieを事前にDB照会しない。
sessionの期限判定、touch、期限切れ削除を本体write transaction内で一度だけ行い、保存結果とともに `Active`、`Inactive`、`NotPresented` のtyped statusを返す。
public writeとaccount履歴のresponseはsession cookieを発行も削除もしない。
古いresponseが並行loginの新cookieを消す競合を避けるため、壊れたcookie、失効cookie、期限切れcookieは履歴で `Expired` として扱い、browser cookieの上書きまたは削除はregister、login、logoutへ限定する。

履歴読込では、sessionの期限判定、touch、期限切れ削除を短いwrite transactionでcommitし、その後に履歴のDEFERRED read transactionを始める。
主催と参加の全件query中にSQLite writerを占有しない。

履歴は主催と参加を別query、別projection、別sectionで返す。
event名、timezone、任意の決定日時、主催履歴の回答件数だけを含み、回答者名、都合、ひとこと、capability、内部IDは含めない。

## UI

`/register`、`/login`、`/history` を追加し、三routeを `noindex,nofollow` にした。
匿名作成画面にはform外の小さな「履歴」linkだけを置き、作成・回答formへlogin fieldやclaim操作を加えていない。

登録とloginはpassword manager用のautocompleteを持ち、pasteを妨げない。
password fieldにはHTMLのUTF-16単位の `maxlength` を置かず、Rustとbrowserで共通のdomain validationがUnicode 128文字と512 octetsを判定する。
field errorは該当inputへ結び付け、request errorは一つのalertとして表示する。
登録画面はpassword再設定がないことと、loginなしでも利用できることを先に説明する。

履歴はSSRでprivate dataを埋め込まず、hydration後に読込中、未login、期限切れ、失敗、認証済みを分ける。
async置換後は履歴見出しへfocusする。
認証済みでは主催と参加を別々に表示し、それぞれ0件の文言を持つ。
logout成功時は同じrouteへの再遷移に頼らず、親のload stateをGuestへ変えてlogin IDとevent名を直ちにDOMから破棄する。
二列は760px以下で一列へ戻り、長いevent名とlogin IDを折り返し、主要操作は44px以上とする。

## 自動検証

```text
cargo test --all-targets
  PASS: default 82 tests

cargo test --all-targets --no-default-features --features server -- \
  --skip simultaneous_identical_retries_create_one_response \
  --skip simultaneous_identical_comments_create_one_value
cargo test --all-targets --all-features -- \
  --skip simultaneous_identical_retries_create_one_response \
  --skip simultaneous_identical_comments_create_one_value
  PASS: 各173 tests

既知の同時再送2件をserver-onlyとall-featuresで各々単独実行
  PASS: 各構成のlogical 175 testsすべて

cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
dx build --web
git diff --check
  PASS
```

Story 8固有では、domain 7件、auth 9件、repository 8件、server 7件、UI 10件とselector単位のresponsive testがPASSした。

## HTTPと稼働状態

- 候補版 `127.0.0.1:8081` と検証済み版 `127.0.0.1:8082` は同時にlistenし、rootはHTTP 200を返した。
- 候補版の `/register`、`/login`、`/history` はHTTP 200を返した。登録とloginのSSRにはautocompleteと `noindex,nofollow`、履歴SSRにはprivate内容を持たないloading stateがあった。
- Originなしのunsafe APIは403と `no-store`、正しいOriginを持つ形式不正APIはmiddlewareを通過して405になった。
- review修正後、auth pathのwrong-method 405にも `no-store` と `nosniff` が付いた。
- 最終方針のmalformed account cookie付きhistory GETは200の `expired` と `no-store`、`nosniff` を返し、`Set-Cookie` を返さなかった。
- 実HTTPでaccountを作成し、`tsunoru-session-local-8081`、HttpOnly、SameSite=Lax、Path=/、30日Max-Ageを確認した。session値自体は記録へ出していない。
- cookie付きでeventを作成すると主催履歴へ一件表示され、logout後の同じcookieは期限切れとして扱われ、browser cookieも削除された。
- 実HTTPで使った生sessionと主催者capabilityは一時fileだけに置き、terminalへ出さず、確認後に削除した。
- review修正後のmalformed-cookie匿名writeは、実SQLiteへ検証rowを書き込む追加承認が得られずHTTPでは再実施していない。parser、session resolution、匿名event・回答transactionの自動testはPASSした。
- 最終方針の反映後、malformed cookieを付けたvalidation失敗のevent作成は、公開write responseへ `Set-Cookie` を含めなかった。成功writeの同じheader境界はsource contractとrepository testで確認し、追加rowを作る実HTTPは行っていない。

## 独立レビュー

認証・Dioxus、SQLite・履歴、UI・accessibilityの三つのread-onlyレビューを実施した。

最初のレビューでは、login試行制限の位置と保持上限、登録の計算予算、session rotationの原子性、malformed cookie cleanup、履歴read中のwriter lock、logout後のprivate DOM、429と通信失敗の案内、Unicode password、route title、responsive testの粒度を指摘された。
ADRとRED testを先に更新し、production実装へ反映した。

再レビューでは、event・回答write前のsession事前照会が本体transactionと重複しているP2を一件指摘された。
typedなsession statusを本体writeから返すよう修正し、再々レビューへ回した。

再々レビューでは、失敗した回答writeでsession cleanupもrollbackする点と、成功した古いwriteのcookie削除responseが並行loginの新cookieを消す点を指摘された。
public writeがcookieを変更しない境界へ改訂したため、失敗writeはcookie cleanupを約束せず、並行responseも新cookieを消さない。
stale cookieが明示的なauth操作までbrowserに残る不利益をADRのconsequenceとして受け入れた。
改訂後の最終データレビューでは、受け入れた方針内に残存P0、P1、P2はなかった。

最終authレビューでは、別tabの古いhistory GETにも同じcookie raceが残るP1と、source contractがendpoint本体を見ていないP2を指摘された。
historyからcookie削除を外し、event作成、回答、historyの各function sectionに `set_session_cookie` と `clear_session_cookie` がないことをREDから固定した。
修正後の最終authレビューでは残存P0、P1、P2はなく、cookie変更はregister、login、logoutの三つに限定されていることを確認した。

UIの残存P2は、logout後の実DOM破棄と `activeElement` を実ブラウザーで確認できていないことである。
passwordの `maxlength` は登録二fieldに加えてlogin fieldも自動testで固定した。
TLS ingress、HTTP redirect、HSTS、backend遮断はdeployment evidenceがなく、公開可能性はUNVERIFIEDのまま残す。

## 実ブラウザー

次はUNVERIFIEDである。

- 320pxとdesktopのcomputed overflow、二列から一列へのreflow、長いlogin IDとevent名。
- keyboardだけでaccount作成、login、履歴link、logout、error後の再試行を操作できること。
- browserのpassword manager、autofill、paste、focus移動、screen reader announcement。
- cookieの実送信、期限切れ表示、logout後の履歴切替、二portを同じbrowserで開いたsession分離。

利用可能なin-app browser clientは起動できず、外部Playwright実行の追加承認も得ていない。

## 一次情報

- [Dioxus 0.7: Authentication](https://dioxuslabs.com/learn/0.7/essentials/fullstack/authentication/): Axum middlewareとserver-only認証状態を組み合わせる根拠。
- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/): typed server functionとrequest contextを使う根拠。
- [RFC 9106: Argon2 Memory-Hard Function](https://www.rfc-editor.org/rfc/rfc9106.html): Argon2id parameterとsaltの根拠。
- [RustCrypto Argon2 0.6.0](https://docs.rs/crate/argon2/0.6.0): Rust実装のPHC、hash、verify APIの根拠。
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html): password保存と計算costの根拠。
- [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html): 一般化したlogin error、長いpassword、試行制限の根拠。
- [OWASP Session Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html): server session、cookie属性、rotation、期限の根拠。
- [OWASP CSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html): target origin検証の根拠。
- [RFC 10025: Cookies](https://www.rfc-editor.org/rfc/rfc10025.html): host-only cookie、`__Host-` prefix、portを分離しないcookie scopeの根拠。
- [RFC 6265 §4.1.1](https://www.rfc-editor.org/rfc/rfc6265.html#section-4.1.1): 並行する `Set-Cookie` responseが到着順のraceになることを判断した根拠。
- [SQLite: ALTER TABLE](https://www.sqlite.org/lang_altertable.html) と [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html): nullable column追加と既存writeへのtransaction統合の根拠。
- [WHATWG HTML: Autofill](https://html.spec.whatwg.org/dev/form-control-infrastructure.html#autofill): username、new-password、current-password tokenの根拠。
- [WCAG 2.2: Accessible Authentication](https://www.w3.org/WAI/WCAG22/Understanding/accessible-authentication-minimum.html): pasteとpassword managerを妨げない根拠。

## 証拠の境界

| 層 | 状態 | 証拠 |
| --- | --- | --- |
| Git | PASS | Story、ADR、RED test、data、server、UIの作業区切りcommit |
| Rust test / lint / format | PASS | default 82件、server各logical 175件、Clippy、format |
| Fullstack build | PASS | client、server成果物 |
| local HTTP | PASS | 8081/8082、auth route SSR、origin拒否、cookie、履歴、logout |
| SQLite | PASS | migration、digest保存、期限・失効、nullable関連、transaction、dedupe |
| Chromium 320px / desktop | UNVERIFIED | browser client起動不良、外部script未承認 |
| password manager / screen reader / physical device | UNVERIFIED | 実端末操作をしていない |
