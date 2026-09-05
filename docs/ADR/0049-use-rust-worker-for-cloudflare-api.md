# ADR 0049: Cloudflare APIをRust Workerで試す

Status: accepted (spike scope)

Date: 2026-09-06

## context

ユーザーは個人開発の学習目的も含め、既存のTypeScript中心の構成とは異なるRust Workerを試したいと決めた。
PR #8では、Rustのdomain検証とArgon2をWasmで動かせること、Dioxus browser側をWasmにできることを確認した。
一方、Dioxus Fullstack serverはTokio/Axum/mioの依存でWorkers向けWasmビルドに失敗した。
Cloudflare公式のworkers-rsはRust Worker、Router、D1 binding、Wasm向け依存を提供する。

## decision

TSUNORUのCloudflare APIはRust Workerとして最小の縦切りを実装し、Dioxusはbrowser UIとして残す。

## rejected options

- TypeScript Workerへ全面移行する。既存Rust domainと学習目的を捨てるため採らない。
- Dioxus Fullstack serverを最初にforkしてWorkers対応する。初回の公開目的にframework保守を持ち込むため別Issue候補とする。
- Rust Workerで最初から全機能を移植する。失敗時の切り戻しが難しく、認証とD1整合性の評価前に範囲を広げるため採らない。

## consequences

RustコードとDioxus UIを再利用できる一方、workers-rsのWasm制約、worker-build、Rust APIとD1の書き換えを学習・保守する必要がある。
API契約、認証、D1操作は本体のSQLx実装と並存する。
最初はhealth、イベント作成、匿名回答だけを試し、移植性や運用負担が大きければTypeScript Workerまたは別ホストへ戻す。
本番DB、既存Cloudflare resource、secret、公開ドメインはこのADRだけでは変更しない。
