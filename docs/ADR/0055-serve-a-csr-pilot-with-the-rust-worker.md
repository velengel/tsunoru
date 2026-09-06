# ADR 0055: 限定版の Dioxus CSR と Rust API を同じ Worker から配信する

## context

Koji Todo は Worker と Static Assets を同居させており、同じ運用を使える。既存 Dioxus の controller は native の account や回答後一覧まで要求し、最小 API へそのまま向けられない。Dioxus 0.7.10 の CLI は dependency の fullstack feature を検出すると server build も追加する。

## decision

既存表示部品を共有した `cloud-web` CSR を独立した feature で構築し、既存アカウントの専用 Rust Worker と同じ origin で配信する。

## rejected options

- native server function を全部 Worker へ再実装する。履歴や account 等まで範囲が広がる。
- React/TypeScript へ画面を書き直す。Rust と Dioxus を試すユーザーの方針に不要な変更となる。
- Pages と別 origin の API を増やす。現段階で CORS と別々の配置管理を増やす必要がない。

## consequences

native-fullstack を default に残し、cloud-web は no-default-features で構築する。作成・回答・主催者集計の controller は限定版に分けるため、移植していない機能を画面で案内しない。表示部品を抽出し、既存 test の入口は再 export で維持する。

Static Assets の SPA fallback が API 拒否を置き換えないよう Worker-first routing を設定する。asset に秘密や利用者データを含めず、API の no-store と静的ファイルの配信を区別する。

Worker と assets は一時ディレクトリでまとめて構築し、成功後に `build/` を置き換える。失敗・中断では途中の出力を回収し、直前の完成物を保つ。ファイルシステムの異常で復元できない場合は、唯一のバックアップを消さずに場所を報告する。コンパイラーの cache は保持する。

根拠: [Static Assets routing](https://developers.cloudflare.com/workers/static-assets/routing/worker-script/)、[SPA routing](https://developers.cloudflare.com/workers/static-assets/routing/single-page-application/)、[Dioxus build](https://dioxuslabs.com/learn/0.7/essentials/fullstack/project_setup/)。既存運用は `koji-todo/wrangler.jsonc`、`voice-workbench/cloud/worker/wrangler.jsonc`。
