# ADR 0015: account履歴を任意のserver sessionへ結び付ける

Status: accepted

Date: 2026-09-02

## context

Product Story 8では、ログイン中に主催または回答したeventへ、後から履歴を通じて戻れるようにする。
一方、TSUNORUの中心は、ログインせずeventを作成し、共有URLから短く回答できる流れである。
accountはこの流れの入口や認可を置き換えず、使う過程で自然に生まれた履歴への任意の戻り道に留める。

現在の主催者専用操作は、accountではなくeventごとの主催者capabilityで認可する。
回答も、回答直後の任意操作だけに使う回答capabilityを持つ。
account sessionをこれらの代わりにすると、履歴の追加だけで既存の公開範囲と権限が変わる。

Dioxus Fullstackは認証機能を内蔵せず、Axum middleware、server-only extractor、DB sessionなどを組み合わせる前提である。
Story 8が必要とする状態はaccount ID、期限、失効だけなので、既存のSQLx poolに狭いrepositoryを追加できる。

password認証には、hash、入力上限、計算資源、account列挙、試行制限、回復手段をまとめて考える必要がある。
session cookieを導入すると、既存の匿名作成と回答も、login中にはaccount履歴を更新するcookie認証writeになる。
そのため、SameSite属性だけでなく、cross-site requestを拒否する境界が必要になる。

local HTTPとproduction HTTPSではcookieの `Secure` 条件が異なる。
さらにcookieはportを分離しないため、同じhostの8081と8082を同時に動かす検証環境では、固定cookie名が互いのsessionを上書きする。

参考資料:

- [Dioxus 0.7: Authentication](https://dioxuslabs.com/learn/0.7/essentials/fullstack/authentication/)
- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/)
- [RFC 9106: Argon2 Memory-Hard Function](https://www.rfc-editor.org/rfc/rfc9106.html)
- [RustCrypto Argon2 0.6.0](https://docs.rs/crate/argon2/0.6.0)
- [OWASP: Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [OWASP: Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)
- [OWASP: Session Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)
- [OWASP: Cross-Site Request Forgery Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html)
- [RFC 10025: Cookies](https://www.rfc-editor.org/rfc/rfc10025.html)
- [SQLite: ALTER TABLE](https://www.sqlite.org/lang_altertable.html)
- [SQLite: Transactions](https://www.sqlite.org/lang_transaction.html)
- [WHATWG HTML: Autofill](https://html.spec.whatwg.org/dev/form-control-infrastructure.html#autofill)
- [WCAG 2.2: Accessible Authentication](https://www.w3.org/WAI/WCAG22/Understanding/accessible-authentication-minimum.html)

## decision

- login IDとpasswordだけを持つlocal accountを実装する。OAuth、email、外部identity providerを初期account作成の前提にしない。
- account作成、login、logout、履歴のため、既存のDioxus typed server functionとSQLx poolを使う。汎用auth frameworkは追加しない。
- login IDは前後の空白を除きASCII小文字へ正規化する。3文字以上32文字以下、先頭は英数字、残りはASCII英数字、period、underscore、hyphenだけを許す。
- passwordは前後を削らず、Unicodeと空白を許す。15文字以上128文字以下、かつUTF-8で512 octets以下とし、文字種の組合せは要求しない。
- account作成ではpassword確認を要求する。password manager、paste、autofillを妨げず、登録には `new-password`、loginには `current-password` を使う。
- HTMLの `maxlength` はUTF-16 code unitで数え、RustのUnicode scalar数と一致しないため、password fieldには付けない。128文字と512 octetsの上限はclientとserverのdomain validationを正とする。
- passwordはArgon2id version 19、memory 19,456 KiB、iteration 2、parallelism 1、output 32 bytes、毎回生成する16-byte以上のsaltでhashし、PHC stringだけを保存する。
- hashとverifyはSQLite transactionの外で、上限4件の `spawn_blocking` taskとして実行する。同時に使うmemoryを約76 MiBへ抑えるためである。
- 存在しないlogin IDでも同じparameterのdummy hashをverifyする。account不存在、password誤り、利用不能なaccountは、同じstatusと一般的な文言で失敗させる。
- login試行は、正規化login IDのSHA-256 hashごとにprocess内で制限する。DB lookup、dummy hash、Argon2 verifyより前に一回を予約し、15分に5回を越えたら、正しいpasswordでも一般的な429と `Retry-After` を返す。成功時だけそのIDのwindowを解除する。
- login limiterは最大4,096 IDだけを保持し、期限切れwindowを削除する。上限中の未知IDはArgon2へ進めず429にする。生login IDを保持しない。
- account作成もhash前にprocess全体のattemptを予約し、15分に100回を越えたら429にする。信頼できるclient addressをapplicationから得られない初期構成なので、local resourceを守る粗い上限とする。
- 初期の試行制限はprocess再起動で消え、複数instanceへ共有されず、ID容量または登録全体の予算を他人が使い切るDoSも残る。public deployment前に、信頼できるclient addressを使うTLS ingressまたは永続rate limitへ置き換える。
- account作成成功時とlogin成功時に、二つのUUIDv4を連結した64文字のrandom session tokenを新しく発行する。browserへ生値を渡し、SQLiteにはSHA-256の32-byte hashだけを保存する。
- 一つのaccountは複数端末のsessionを持てる。同じbrowserでaccount作成またはloginをし直す場合は、届いた旧sessionのDELETEと新sessionのINSERTを同じ `BEGIN IMMEDIATE` transactionでcommitしてから、新cookieを発行する。旧sessionを削除できなければ新sessionも発行しない。
- sessionは7日のidle timeoutと30日のabsolute timeoutをserver時刻で判定する。`last_seen_at` は一時間以上空いた認証済みrequestでだけ更新し、SQLite writeを抑える。
- logoutはserver側のsession rowを削除し、同じcookieを期限切れにする。logout後に始まるrequestは必ず拒否する。logoutと既に進行中のrequestが競合した場合、先に認証transactionを成立させた一件は完了し得る。
- production cookieは `__Host-tsunoru-session` とし、`Secure`、`HttpOnly`、`SameSite=Lax`、`Path=/`、`Max-Age=2592000` を付け、`Domain` は付けない。
- local HTTPではloopback hostだけを許し、`Secure` を付けない。cookie名は `tsunoru-session-local-{port}` として8081、8082などを分離し、それ以外の属性はproductionと揃える。
- Dioxus開発proxyは外側のportをbackendの一時portへ置き換え、元のHost headerを転送しない。backendのHostがloopbackであることを確認した上で、CLIが注入する `DIOXUS_DEVSERVER_PORT` を外側portとしてlocal originとcookie名に使う。
- productionのpublic originは `TSUNORU_PUBLIC_ORIGIN` に完全なHTTPS originとして明示する。未設定時はloopback HTTPだけを許す。forwarded headerや任意の `Host` からproduction originを推測しない。
- `TSUNORU_PUBLIC_ORIGIN` はTLSを終端しない。internetへ公開する場合は、TLSを終端しHTTPをHTTPSへredirectしてHSTSを返すingressの背後へ置き、DioxusのHTTP listenerへinternetから直接到達できない構成を別途必須にする。そのdeployment evidenceがない限りaccount機能をpublic-readyと扱わない。
- `POST`、`PUT`、`PATCH`、`DELETE` の `/api/` requestは、`Origin` が設定済みpublic originと完全一致する場合だけ許す。`Origin` がない場合に限り同一originの `Referer` を認め、両方がない、`null`、または不一致ならdecode前に403とする。
- CSRF拒否、auth、private historyのresponseへ `Cache-Control: no-store` と `X-Content-Type-Options: nosniff` を付ける。authとprivate historyはAxum middlewareでもpathを判定し、server functionへ入る前のrouting・decode errorにも同じheaderを付ける。認証errorへ入力値、accountの有無、password、session token、内部hashを含めない。
- SQLiteへ `accounts` と `account_sessions` を追加する。`events.organizer_account_id` と `responses.respondent_account_id` はnullable foreign keyとし、account削除時は `SET NULL`、sessionは `CASCADE` とする。
- nullable foreign keyを使うのは、初期modelが一eventにつき主催accountは最大一つ、一responseにつき回答accountは最大一つだからである。汎用activity logや履歴snapshotを二重書きしない。
- clientからaccount IDを受け取らない。serverがcookieのsession tokenをhashし、同じwrite transaction内で有効なaccount IDを解決する。
- 形が正しいsession cookieをevent作成・回答の前段でDB照会しない。repositoryは既存のwrite transaction内でsessionを一度だけ解決し、保存値とともに `Active`、`Inactive`、`NotPresented` のtyped statusを返す。
- login中のevent作成では、eventと候補と `organizer_account_id` を同じ `BEGIN IMMEDIATE` transactionで保存する。
- login中の新規回答では、responseと都合と `respondent_account_id` を同じ `BEGIN IMMEDIATE` transactionで保存する。
- cookieがない、形が壊れている、失効済み、期限切れの場合は匿名writeとして続ける。account sessionが匿名操作の前提にならないためである。
- event作成、回答、account履歴のresponseは、session cookieを発行も削除もしない。古いresponseによるcookie削除と、新しいlogin responseによるcookie発行が並行すると、到着順によって新sessionまで削除し得るためである。履歴は壊れた、失効済み、期限切れのcookieを `Expired` stateとして扱い、browser cookieの変更はregister、login、logoutの明示的なauth操作へ限定する。
- 同じcapabilityによる保存済み回答の再試行では、既存のaccount関連付けを変更しない。最初の保存が匿名なら、後からloginして同じrequestを送っても匿名のままにする。
- Story 8以前のrow、login前のrow、respondent nameやlogin IDが似たrowを遡ってaccountへ結び付けない。明示的なcapabilityによるclaimもStory 8では実装しない。
- account sessionは主催者capabilityと回答capabilityの代わりにしない。履歴からは既存のpublic eventへだけ戻し、主催者用summaryへ直接linkしない。
- `/register`、`/login`、`/history` を追加し、authとhistory routeには `noindex,nofollow` を指定する。作成画面の上部には小さな履歴linkだけを置き、作成form、公開event、回答form、回答完了へloginやclaimの操作を加えない。
- history projectionは主催と参加を別の型、別のsectionで返す。event名、event timezone、任意の決定日時、主催履歴の回答aggregate件数だけを含める。
- history projectionにaccount ID、内部ID、主催者のひとこと、回答者名、都合、回答者のひとこと、各種capabilityとhashを含めない。
- 主催履歴はevent作成時刻とpublic ID、参加履歴は最後に成立した本人のresponse IDとpublic IDで、いずれも新しい順へ安定させる。同じeventを複数回回答しても参加履歴では一件にまとめる。
- 同じaccountが一つのeventを主催し回答もした場合は、役割が異なるため両方へ表示する。
- sessionの期限判定、必要なtouch、期限切れrow削除は短い `BEGIN IMMEDIATE` でcommitし、その後に二つの一覧を一つのDEFERRED read transactionで読む。履歴全件を読む間は匿名作成・回答が必要とするwriterを占有しない。認証成立後にlogoutが競合したreadは完了し得る。
- 二つの一覧は一つのread transactionで読む。初期MVPは全件を返し、paginationをまだ導入しない。個人履歴が増えた場合のquery時間とresponse sizeを受け入れ、public deployment前にkeyset paginationを再検討する。
- history UIは未login、loading、主催0件、参加0件、読込失敗、session期限切れ、logout中、logout失敗を区別する。mobileでは一列、十分な幅では二列にしてもDOM順は主催、参加のまま保つ。async読込後は履歴見出しへfocusし、logout成功時は同じrouteへの遷移へ任せずclient stateからlogin IDとevent名を直ちに破棄して未login表示へ変える。
- logout時に `Clear-Site-Data` やbrowser storageの一括削除を行わない。既存の匿名主催者capabilityまで失わせないためである。
- password reset、login ID recovery、email確認、MFA、account削除はStory 8へ含めない。passwordを失うとaccount履歴を回復できないことを登録前に説明し、public deployment前に回復方法を再判断する。

## rejected options

### accountを匿名作成・回答の入口にする

履歴との関連付けは分かりやすくなる。
しかし、TSUNORUの中心であるログインなしの最短経路を劣化させるため採用しない。

### OAuth、magic link、passkeyを最初の認証方式にする

password保存を避けられる方式はある。
しかし、OAuthはprovider設定とsecret、magic linkはmail配送、passkeyはRP domainと回復設計を必要とし、zero-configurationのlocal serverでStory 8を動かす条件に合わない。

### 汎用auth frameworkを追加する

middleware、session rotation、複数backendなどをまとめて利用できる。
しかし、Story 8で使う状態より表現範囲が広く、既存のSQLx transactionへevent・response関連付けを統合する境界が見えにくくなるため採用しない。

### JWTだけでsessionを保持する

requestごとのDB lookupを省ける。
しかし、logout、期限切れ前の失効、盗難tokenの無効化を即時に反映できないため採用しない。

### passwordを高速hashまたは暗号化して保存する

実装とverifyは短くなる。
しかし、DB漏えい時のoffline試行を十分遅くできず、復号可能な保存も不要なため採用しない。

### 固定名のcookieをlocalとproductionで共用する

設定とtestは少なくなる。
しかし、cookieはportを区別せず、8081と8082が互いのsessionを上書きする。localはport別名、productionは `__Host-` 名に分ける。

### password fieldへdomainと同じ128のmaxlengthを付ける

browser側で過大入力を早く止められる。
しかし、HTMLはUTF-16 code unit、RustはUnicode scalarで数えるため、絵文字などdomain上は有効なpasswordをbrowserだけが拒否する。HTML上限は置かず、共通domain validationで案内する。

### public originの設定だけでHTTPS運用済みとみなす

Secure cookieとOrigin照合はHTTPS originを前提にできる。
しかし、環境変数はTLS終端、HTTP redirect、HSTS、backend遮断を提供せず、公開されたHTTP login画面からpasswordを平文送信できる。deployment側のTLS証拠を別の必須条件にする。

### 新sessionをcommitしてから旧sessionをbest-effortで削除する

新しいlogin自体はDB削除失敗から独立できる。
しかし、旧tokenを盗まれていた場合に二つのsessionが有効になり、rotationの意味を失う。旧DELETEと新INSERTのどちらか一方だけをcommitしない。

### SameSiteだけをCSRF対策にする

通常のcross-site form送信の多くを抑えられる。
しかし、login CSRFを含むcookie付きwriteの境界を一属性へ委ね、将来のbrowser挙動や同一site構成へ弱いため、target originの照合も行う。

### 汎用の履歴tableまたはevent snapshotへ二重書きする

将来のactivity feedやrevision表示へ広げやすい。
しかし、現在の正本と履歴がずれる部分成功を増やす。eventとresponseのnullable account foreign keyから最小projectionを読む。

### 名前やlogin IDから過去の匿名利用を自動claimする

登録直後から履歴を多く見せられる。
しかし、同名利用者や共有端末から他人のeventを奪えるため採用しない。

### capabilityを提示した過去eventのclaimをStory 8へ含める

主催者については推測せず過去eventを回収できる。
しかし、回答capabilityは回答直後のmemoryにしかなく役割間で非対称になり、accountと既存認可の説明も増えるため後続判断へ送る。

### account sessionを主催者capabilityの代わりにする

履歴から主催者画面へ直接戻れる。
しかし、別端末のaccountへ既存eventのprivate権限を拡大し、Story 8の履歴追加を越えるため採用しない。

### logout時にbrowser storageを一括削除する

端末に残る状態をまとめて消せる。
しかし、account sessionと無関係な匿名主催者capabilityも消し、eventを管理できなくなるため採用しない。

### public writeまたは履歴の応答で古いsession cookieを削除する

event作成、回答、履歴読込のついでに、壊れた、失効済み、期限切れのcookieをbrowserから掃除できる。
しかし、古いrequestとloginが並行すると、遅れて届いた削除responseがlogin直後の新cookieを同じ名前で消し得る。
HTTP cookieには届いた値と同じ場合だけ削除する条件付き更新がないため、これらのresponseでは `Set-Cookie` 自体を返さない。

## consequences

- accountを使わない利用者は、従来と同じfieldと操作数でevent作成と回答を完了できる。
- login中の利用者だけは、追加入力なしで主催・参加履歴が蓄積する。
- Story 8より前、session期限切れ後、login前の匿名利用は履歴へ現れない。後から同名accountを作っても回収されない。
- accountとeventの関連付けは履歴を読む根拠であり、主催者専用操作の認可にはならない。
- account削除を将来実装しても、nullable foreign keyを外すだけでpublic event、回答、決定は残る。実際の削除UIでは、この保持方針を利用者へ説明する必要がある。
- password hash一件につき約19 MiBを使い、同時処理上限でも約76 MiBを使う。速いhashよりloginは遅くなるが、offline攻撃への抵抗を優先する。
- 7日idle、30日absoluteのsessionは履歴へ戻りやすい一方、盗難cookieが使える時間を長くする。public deployment前に扱う情報と再認証要件から再評価する。
- localの非Secure cookieはloopback開発だけの例外である。LAN上のHTTP authは対応せず、productionでは明示したHTTPS originに加え、TLS ingress、HTTP redirect、HSTS、backend遮断の運用が必要になる。
- local originの復元はDioxus CLIが管理する環境変数に依存する。CLIを介さず別proxyから起動する場合は `TSUNORU_PUBLIC_ORIGIN` を明示する。
- Origin検証により、browser以外のAPI clientと既存のOriginなしPOSTはそのまま使えない。testと手動HTTP検証は正しいOriginを明示する。
- process内のlogin・登録試行制限は単一instanceの初期防御に留まる。固定ID容量と登録全体の予算はmemoryとArgon2 queueを抑える一方、再起動、水平分割、分散送信元、予算枯渇によるDoSに耐える公開用rate limitではない。
- sessionの `last_seen_at` 更新はSQLite writeを増やす。一時間以内のrequestでは更新を省き、event・response writeでは既存transactionへ含める。
- event・response writeがsessionの有効性をtyped statusとして返すため、serverとrepositoryの型が一段増える。代わりにsessionの事前照会による二重writer lockを持ち込まず、transaction内で成立した関連付けをtestから確認できる。
- 壊れた、失効済み、期限切れのcookieはbrowserに残り、public writeと履歴でrequestごとの判定が増える。register、login、logoutのいずれかを完了すると上書きまたは削除される。login直後の新cookieを古い並行responseが消す危険を避ける方を優先する。
- 履歴を全件返す初期実装は単純だが、長期間使うaccountでは大きくなる。履歴量を測り、表示を失わないkeyset paginationを後から追加する。
- 履歴のsession判定と一覧snapshotは別transactionなので、その間のlogout後も認証済みreadが一度完了し得る。長い一覧readが匿名writeのwriterを占有しない方を優先する。
- nullable foreign keyは一event一主催account、一response一回答accountを固定する。共同主催が必要になれば関連tableへのmigrationが必要になる。
- password回復、MFA、email確認、永続rate limitがないため、この初期認証をそのままinternet公開の完成形とは扱わない。
