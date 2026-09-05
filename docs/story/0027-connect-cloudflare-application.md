# Story 0027: Dioxus画面とCloudflare APIを接続する

## context

ユーザーは小実験に続き、認証、画面接続、Cloudflare上の動作確認まで進めるよう依頼した。
未マージPR #8と同じworktreeを継続し、実験結果を消さずに接続実装を追加する。

## definition of done

- [ ] 既存Dioxus画面から登録、login、作成、匿名回答、主催確認をD1へ接続する。
- [ ] 公開projectionと主催capability、回答capability、sessionの認可を守る。
- [ ] local API負例とbrowser導線、Cloudflare上の動作を区別して記録する。
- [ ] 未対応機能を成功とせず別Issueへ分離し、self-reviewと修正を収束させる。

## to do

- [ ] Story/ADR、HTTP契約試験を実装前に置く。
- [ ] Rust共通検証とArgon2 PHCをWasm化し、Workerと実schemaのD1へ接続する。
- [ ] 静的Dioxus build、ローカル検証、隔離したCloudflare検証環境への反映を行う。

## concern

既存アプリの本番DB、secret、Workerを変更しない。新しいTSUNORU検証用資源だけを使う。
小実験用の固定session/salt/APIを公開実装に流用しない。
初回接続の目的を越える独立機能は必要性を判断して分割する。
