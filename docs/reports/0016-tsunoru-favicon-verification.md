# Story 0016 検証記録

Date: 2026-09-02

対象commit: `38e36c3 docs: define the TSUNORU favicon identity`、`d485445 test: require a bundled TSUNORU favicon`、`35cf46f feat: add the TSUNORU gathering favicon`

## 結論

TSUNORUのfaviconを作成し、Dioxusの共通document headへ設定した。

三つの生成りの点と経路が一つの橙色の点へ集まる形で、参加者の都合をつのって一つの開催日を決める流れを表す。
深緑、生成り、橙色は既存画面の主要色と同系である。

source asset、Dioxusのhash付きbuild asset、実HTTP、64px・32px・16pxの縮小像、自動test、Clippy、format、Fullstack buildはPASSした。
実ブラウザーのタブ表示はbrowser接続を初期化できなかったためUNVERIFIEDである。

## test-firstの証拠

Story 0016、ADR 0022、用語集を先に作成した。
続いて、Dioxusのasset参照、document headの `icon` link、64px RGB PNGを要求するtestを追加した。

実装前の `cargo test --test favicon` は2件ともFAILした。
一件は `FAVICON` assetが未定義、もう一件は `assets/favicon.png` が存在しないことを検出した。

faviconとdocument linkを追加した後、同じ2件はPASSした。

## favicon asset

- source：`assets/favicon.png`
- 形式：64px × 64px、8-bit RGB PNG
- 大きさ：4,769 bytes
- 64px、32px、16pxへ縮小した実画像で、三つの点、三本の経路、橙色の到達点を目視した。
- 16pxでも三つの点と一つの到達点を区別できた。

最終assetは組み込みのImageGenで生成し、`sips` で64pxへ縮小した。
生成時には、深緑を外周まで敷く不透明な正方形、三つの生成りの点と経路、一つの橙色の点、中央配置、太い線、文字とカレンダー格子を含めないことを指定した。

## Dioxusと実HTTP

`src/lib.rs` は `asset!("/assets/favicon.png")` を定義し、共通document headから `rel="icon"` で参照する。

分離したbuild出力とsession cacheを使って `127.0.0.1:8092` に検証用Fullstack serverを起動した。
rootはHTTP 200を返し、HTMLは次のhash付き参照を含んだ。

```html
<link rel="icon" href="/assets/favicon-dxh9c1eb1a55015dd57.png"/>
```

配信assetは64px × 64pxの8-bit RGB PNGだった。
Dioxusはsource PNGの色管理metadataを除いて配信したためbyte hashとfile sizeは変わったが、FFmpegのSSIMはRGB全channelで1.000000となり、全画素が一致した。

Dioxus asset pipelineの採用根拠は[Dioxus 0.7 Assets](https://dioxuslabs.com/learn/0.7/essentials/ui/assets/)である。
内容に対応するhash付きpathをdocumentへ挿入できる。

## 自動検証

```text
cargo test --test favicon
cargo test --all-targets
cargo test --all-targets --features server
cargo test --all-targets --no-default-features --features server
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
dx build --web
git diff --check
  PASS
```

`dx build --web` はclientとserverを構築し、client成果物へhash付きfaviconをcopyした。

## designer review

faviconは64px、32px、16pxで主要なシルエットと色の区別を保った。
文字、細線、操作controlを追加していない。
document headだけの変更であり、既存layoutとfocus順を変更しない。
既存の320px responsive contract testを含む通常構成とserver構成の全testはPASSした。

ただし、実ブラウザーの320pxとdesktop viewport、明るいtabと暗いtab、browser chrome内の実表示は確認していない。
in-app Browserは、plugin runtimeのtrusted path errorで初期化できなかった。

## 証拠の境界

| 層 | 状態 | 証拠 |
| --- | --- | --- |
| favicon source | PASS | 64px RGB PNG、32px・16px縮小像 |
| Rust test / lint / format | PASS | default、server、server-only、Clippy、format |
| Fullstack build | PASS | client、server、hash付きfavicon |
| local HTTP | PASS | root 200、document link、64px配信asset、SSIM 1.000000 |
| Chromium 320px / desktop | UNVERIFIED | browser plugin runtimeを初期化できなかった |
| external deployment | UNVERIFIED | deployしていない |
| physical device | UNVERIFIED | 実端末で確認していない |
