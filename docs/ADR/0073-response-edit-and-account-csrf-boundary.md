# ADR 0073: 回答編集とaccount/CSRFの境界を限定する

## context
回答編集はcapabilityで実装済みだが、account全面導入は匿名回答の導線とCSRF境界を同時に変える。

## decision
回答編集は未実装として後続へ延期し、account全面移行とは分離する。現行organizer CookieのCSRF検証は公開前条件として維持する。

## rejected options
今回の公開前判断でaccount/Cookieを全面導入する案は、別の認証変更を同時に持ち込むため退ける。

## consequences
回答編集は変更内容を受け付けず、実装・検証まで公開条件を満たさない。organizer Cookieのnegative-origin検証を継続する。
