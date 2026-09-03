# ADR 0005: 公開イベントと主催者権限を分離する

Status: accepted

Date: 2026-09-01

## context

TSUNORUの主要フローは、ログインなしでイベントを作り、回答し、主催者が日程を決める。
回答者へ渡す共有URLに主催者権限まで含めると、そのURLを受け取った全員が日程を決定できてしまう。

ログインを必須にせず主催者だけの操作を守るには、イベントを見るための識別子と、主催者であることを示す秘密を分ける必要がある。

## decision

- 共有URLは、高いランダム性を持つ公開イベントIDを使う `/events/{public_id}` とする。
- 共有URLを知る人は、ログインなしでイベントを閲覧し、後続Storyで回答できる。これを `public-by-link` と呼ぶ。
- 公開イベントIDは到達しにくくするための識別子であり、認可用の秘密としては扱わない。
- イベント作成時に、公開イベントIDとは別の主催者capabilityをサーバーで生成する。
- DBには主催者capabilityのSHA-256 hashだけを保存し、生の値は作成時に一度だけブラウザーへ返す。
- ブラウザーは主催者capabilityをevent単位で `localStorage` に保存し、主催者専用server functionを呼ぶときだけ送る。
- `localStorage` への保存に失敗した場合は、作成成功を取り消せないため、生のcapabilityをその成功画面に限って復旧キーとして表示する。利用者が手動で保存できるまで、成功表示だけに切り替えて値を捨てない。
- 回答用の共有URL、server log、DBへ生の主催者capabilityを含めない。
- 共有イベント画面は `robots` の `noindex, nofollow` を宣言し、検索公開を意図しないことをcrawlerへ伝える。ただし、これは認可や秘密保持の仕組みとしては扱わない。
- 存在しない公開イベントIDをSSRで開いた場合は、Dioxusの `FullstackContext` からHTTP 404を確定し、見た目だけでなくHTTP semanticsでも未発見を表す。
- 主催者専用操作は、型付きserver functionであっても、サーバー側でcapabilityを必ず検証する。
- Product Story 1では共有URLだけを利用者へ明示し、主催者capabilityの手入力を要求しない。

参考資料：

- [Dioxus 0.7: Server Functions](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/)
- [WHATWG: Web Storage](https://html.spec.whatwg.org/multipage/webstorage.html)
- [RFC 3986: Fragment](https://www.rfc-editor.org/rfc/rfc3986.html#section-3.5)

## rejected options

### 共有URLを主催者用URLとしても使う

URLが一つで済む。
しかし、回答者全員へ決定権を渡すため、主催者が決めるというプロダクト原則を守れない。

### 主催者capabilityを共有URLのqueryへ入れる

別端末への復旧リンクを作りやすい。
一方、queryはHTTP request、アクセスlog、referrer、履歴へ残りやすい。
回答用URLの誤共有で主催者権限も漏れるため採用しない。

### 公開イベントIDだけを秘密として扱う

追加のtoken保存が不要になる。
しかし、閲覧と管理を同じ秘密にすると、回答者へ閲覧権を渡す時点で管理権限も渡るため採用しない。

### Product Story 1からログインを必須にする

一般的なsessionとaccount ownershipで主催者権限を表せる。
主要フローをログインなしで完遂するというFirst Instructionに反するため却下する。

### 主催者capabilityを平文でDBへ保存する

比較処理は簡単になる。
DBファイルを読まれた場合に主催者操作を直ちに再現できるため採用しない。

### localStorageへの保存失敗を無視する

通常経路の画面は簡単になる。
しかし、イベントはcommit済みで作成し直せず、生のcapabilityもサーバーから再取得できない。
そのまま成功画面へ移ると、利用者が気付かないまま主催者権限を永久に失うため採用しない。

### 存在しないイベントもHTTP 200で表示する

Dioxus Routerの動的routeだけを使う場合は追加処理が要らない。
しかし、crawler、監視、共有URLを検査するclientが、存在する画面と未発見を区別できない。
SSR中にFullstackのresponse statusを設定できるため、見た目だけの未発見表示にはしない。

## consequences

- 回答者へ渡すURLだけでは、主催者専用の日程決定を実行できない。
- ログインなしの最短経路と、最小限の主催者認可を両立できる。
- `localStorage` は同じoriginのscriptから読めるため、XSS対策が主催者権限の保護にもなる。
- ブラウザーの保存内容を削除すると主催者capabilityを失う。作成時の保存失敗には復旧キー表示で対処するが、後日の削除に対する復旧方法はログイン機能またはfragmentを使う管理リンクの後続判断が必要になる。
- 復旧キーを表示する例外経路では、画面を見られる人やスクリーンショットから主催者権限が漏れ得る。回答用URLとは明確に分け、この画面を閉じる前の保存を促す。
- public-by-linkは完全な非公開ではない。URLを知る人による閲覧を許容できない情報は保存しない必要がある。
- `noindex` はcrawlerへの指示であり強制力はない。共有URLが漏れた場合の閲覧を防ぐものではない。
- HTTP 404を設定できるのは新規document requestのSSR時である。hydrate後のclient-side遷移では画面上の未発見表示が境界となる。
- SHA-256 hashの比較だけではtokenの失効、再発行、複数端末管理を扱えない。必要になるまでは初期MVPへ追加しない。
