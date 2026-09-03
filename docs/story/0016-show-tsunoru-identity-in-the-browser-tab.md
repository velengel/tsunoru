# Story 0016: ブラウザーのタブでTSUNORUを見分けられる

Status: complete

Date: 2026-09-02

## context

TSUNORUの画面にはプロダクト名と固有の配色があるが、ブラウザーのタブには標準の無地アイコンしか表示されない。
複数のページを開いた利用者は、文字を読むまでTSUNORUのタブを見分けられない。

TSUNORUは、参加者から都合をつのり、主催者が一つの開催日を決める日程調整アプリケーションである。
この流れを、複数の点が一つの場所へ集まる小さなマークとして表す。

## definition of done

- TSUNORUのコンセプトと画面配色に沿う固有のfaviconを表示する。
- faviconは、複数人の都合が一つの開催日へ集まる構造を、文字に依存せず表す。
- 16px相当まで縮小しても、主要な点とシルエットを見分けられる。
- faviconは深緑を外周まで敷き、明るいタブと暗いタブのどちらでも一貫した背景色を保つ。
- Dioxusのasset pipelineからfaviconを配信し、内容変更時にはhash付きURLが更新される。
- 利用者に届くdocument headがfaviconを参照することをtestで固定する。
- test、Clippy、format、Fullstack build、秘密情報検査が成功する。

## to do

- [x] TSUNORUのコンセプト、配色、document head、asset pipelineを確認する。
- [x] faviconの意味、表現、配信方法をADRへ記録する。
- [x] faviconの実体とdocument head参照を要求する、失敗するtestを書く。
- [x] faviconを生成し、64pxの不透明PNGとして整える。
- [x] Dioxusのdocument headからfaviconを参照する。
- [x] 64px、32px、16pxの見え方を確認する。
- [x] 全検証を実行し、発見事項を記録する。

## concern

- 64pxのraster画像だけを正本にすると、将来の大判ロゴには流用しにくい。
- 生成画像は細部が均一とは限らないため、縮小後の実画像を確認する必要がある。
- カレンダーだけの記号は日程アプリだと伝わるが、参加者から都合をつのるTSUNORU固有の意味が弱くなる。
- 角を透過したiconよりタブ上の占有面積が大きく見えるため、内部の余白で圧迫感を抑える。
