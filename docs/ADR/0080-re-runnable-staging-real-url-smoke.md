# ADR 0080: staging 実 URL smoke 検証を専用 CLI に固定する

## context

staging の主要な利用導線は実 URL、Worker、D1、ブラウザーの組み合わせで成立する。既存の Miniflare 契約テストだけでは、Custom Domain、TLS、実 Worker 設定、実 D1 の接続を証明できない。手動検証は再現性と後始末の証拠が弱い。

## decision

正規 staging URL に対する health、試用セッション、合成イベントの作成・読取・匿名回答・削除を、秘密値を表示せず失敗時に cleanup を試みる専用 CLI smoke verifier として実装する。

## rejected options

手動の curl 手順は、各操作の期待値と cleanup を一貫して強制できず、再実行時の漏れを招くため採用しない。Miniflare の既存テストだけに依存する案は、実 URL・TLS・配置済み binding を確認できないため採用しない。ブラウザー専用の検証は API の失敗理由と cleanup を機械的に集約しにくいため、CLI を正規の一次検証器とする。

## consequences

実 staging D1 に一時イベントを書き込むため、検証器は予測不能な ID と organizer capability を使い、必ず削除結果を確認する。ネットワーク断などで削除できない場合は失敗として明示し、残存 ID を秘密値なしで報告する。HTTP の実 URL 証拠とブラウザー・実機の証拠は別の検証層として記録する。
