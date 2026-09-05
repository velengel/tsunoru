# ADR 0045: 既存Cloudflare運用を公開構成の第一候補にする

Status: accepted (evaluation order only; supersedes ADR 0044)

Date: 2026-09-05

## context

ユーザーはCloudflareアカウントとkoji-todo/voice-workbenchの実績を優先すると明示した。
ADR 0044はコード変更量を重く見て、既存運用を再利用する価値を評価していなかった。
両repoの設定とCloudflare公式資料を調査したところ、Rust利用そのものはCloudflareを除外する理由にならない。
SQLxのファイルDB、transaction、Dioxus server実行、Argon2の実行方式の適合は小実験が必要である。
詳細は[比較追補](../reports/0022-cloudflare-existing-project-comparison.md)に置く。

## decision

初回公開は既存運用に沿うCloudflare WorkersとD1を第一候補として適合性を検証する。

## rejected options

- Rustであることだけを理由にRenderを先に契約する。公式Rust対応と既存運用の利点を無視する。
- TypeScriptへ全面書き換えする。Rust UI/domainを維持できる可能性があり、現時点で必要性がない。
- Cloudflareへ無変更で配置できると断定する。現行DBとruntimeの互換性は未実証。

## consequences

既存のアカウント、デプロイ、D1運用の経験を使えるが、nativeホストよりコード変更が増える可能性がある。
既存アプリの認証、DB、secret、設定IDは共有せず、TSUNORU用に分離する。
小実験で重大な非互換または費用上の不利が確認された場合だけ、Containers構成やnativeホストを再比較する。
この判断は契約、デプロイ、全面移植の実施を意味しない。
