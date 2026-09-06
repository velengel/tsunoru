# ADR 0060: 主催者だけGoogle認証を必須にし回答者は匿名で許可する

## context

Cloudflare版は現在、共有試用コードで入口を保護している。主催者のイベント作成・集計には本人の継続的な識別が必要だが、回答者にログインを強制すると、共有URLから短く回答する中心体験を損なう。ログインした回答者には、将来、自分の回答履歴を提供したい。

GoogleのOIDC ID Tokenはログイン用途に使える。Google Calendar等のAPI利用を始めるまでは、Googleのrefresh tokenを保持する必要はない。

## decision

Google OIDCで主催者のWorkerセッションを発行し、回答APIは匿名利用を維持する。

## rejected options

- 全員にGoogleログインを要求する：匿名回答の導線を失う。
- Google tokenをアプリの認可に直接使う：アプリ固有のセッション・失効・権限境界を持てない。
- Google Calendar権限まで同時に要求する：ログインに不要なscopeとrefresh token管理を持ち込む。
- 試用コードを本番の主催者認証として残す：共有秘密で個人の所有権を表現できない。

## consequences

主催者の作成・集計・削除はGoogle accountに紐付くHttpOnlyセッションで保護できる。回答者は未ログインで回答でき、任意ログインと履歴は後続Storyで追加する。Google Client IDと検証用設定は公開値として扱えるが、Google Client Secretを採用する場合はWorker Secretへ置く。OAuth client設定、redirect URI、アカウント削除・回復、rate limitは別途運用判断が必要である。
