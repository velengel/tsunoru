# ADR 0072: 旧SQLite移行とorigin越えcapability引き継ぎを行わない

## context
旧SQLiteの所有権・欠損対応と、originを越えた秘密の移送手段は定義されていない。

## decision
旧SQLiteの自動移行とorigin越えcapability引き継ぎは行わない。

## rejected options
自動変換やURL/Cookie引き継ぎは、誤移行と秘密拡散を招くため退ける。

## consequences
旧データは自動でD1に現れず、origin移行時は再作成・再回答が必要になる。
