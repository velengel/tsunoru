# ADR 0013: Dioxusのdecode errorをapplication契約の外側として扱う

## context

Product Story 6のprivate POSTは、成功とapplication errorへ `Cache-Control: no-store` を付け、入力不正を422、認可失敗を404、競合を409として返す。

Dioxus 0.7のserver functionは、Rust関数の引数からserializableなrequest bodyを生成し、登録されたAxum handlerが関数を呼ぶ前にJSONをdecodeする。
実HTTP確認では、生成された `{ "input": ... }` の形を満たして値だけが不正なrequestは関数へ届き、422と `no-store` を返した。
一方、`input` field自体がないwire JSONは関数へ入る前にDioxusが500を返すため、関数内の `FullstackContext` からresponse headerを追加できなかった。

500はHTTP仕様上heuristically cacheableなstatusではなく、このdecode errorはevent、candidate、capability、hash、回答をresponseへ含めない。

参考:

- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/)
- [Dioxus 0.7: Adding a Backend](https://dioxuslabs.com/learn/0.7/tutorial/backend/)
- [Dioxus 0.7 migration: default server function codec](https://dioxuslabs.com/learn/0.7/migration/to_07/#change-default-server-function-codec-to-json)
- [RFC 9110: HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html)
- [RFC 9111: HTTP Caching](https://www.rfc-editor.org/rfc/rfc9111.html)

## decision

- Story 6のHTTP契約は、Dioxusが生成するtyped clientまたは同じwire shapeから関数へ到達したrequestを対象とする。
- 構造が正しく値だけが不正なrequest、認可失敗、candidate不一致、競合、DB失敗には、関数内で明示したstatusと `Cache-Control: no-store` を保つ。
- handler前のJSON decode errorは、Dioxus 0.7が生成するtransport errorとしてapplication errorと分ける。Story 6ではcustom Axum middlewareや手書きrouteを追加せず、500かつ秘密情報を含まない現在の挙動を受け入れる。
- 外部APIとして第三者clientへwire protocolを公開する場合、またはserver functionのdecode errorにも独自statusとheaderが必要になった場合に、この判断を見直す。

## rejected options

### server functionを手書きのAxum routeへ置き換える

decode前からheaderとstatusを制御できるが、Rust frontendとserver間の型付きclient生成を失い、同じrequestとresponseのserialize境界を二重管理する。
一つの匿名MVP操作のために通信方式全体を分岐させない。

### Dioxus router全体へprivate API専用middlewareを追加する

pre-handler responseへ `no-store` を付けられる可能性はあるが、現在の `dioxus::launch` からcustom Axum起動へ構成を広げる必要がある。
500はheuristic cache対象でなくprivate payloadも返さないため、Story 6では釣り合わない。

### decode errorを422として扱えたと報告する

関数へ到達する形式不正値と、関数へ到達しないwire不正を混同する。
実HTTPの500という観測結果を隠すため採用しない。

## consequences

- Dioxusのtyped clientから送る通常requestと、同じwire shapeを使う実HTTP requestは、422、404、409、500と `no-store` のapplication契約を受け取る。
- `input` field欠落など生成wire protocol自体を満たさないrequestは、Dioxus 0.7のdecode errorとして500になり、関数内で付ける `no-store` は付かない。
- 500はheuristically cacheableではなく、POST responseにも明示的なfreshnessがないため、今回の受容でprivate event dataをcache可能にするわけではない。
- framework更新でdecode errorのstatus、body、headerが変わり得る。実HTTP検証ではapplication errorとtransport decode errorを分けて記録する。
- 将来custom routerを導入する際は、private API pathへ共通 `no-store` middlewareを置けるか再評価する。
