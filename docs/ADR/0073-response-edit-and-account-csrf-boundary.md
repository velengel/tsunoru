# ADR 0073: 回答編集とaccount/CSRFの境界を限定する

## context
回答編集はcapabilityで実装済みだが、account全面導入は匿名回答の導線とCSRF境界を同時に変える。

## decision
回答編集は既存capability境界で提供し、account/CSRF全面移行は匿名導線を保つ後続判断へ分離する。

## rejected options
今回の公開前判断でaccount/Cookieを全面導入する案は、別の認証変更を同時に持ち込むため退ける。

## consequences
回答編集は利用者別失効を持たず、account/CSRF統合は未実装として公開条件から分離される。
