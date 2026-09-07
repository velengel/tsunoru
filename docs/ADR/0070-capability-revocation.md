# ADR 0070: 利用者別capability失効を公開前に実装する

## context
現行capabilityは個別失効を提供しない。

## decision
利用者別capability失効を実装・検証するまで一般公開しない。

## rejected options
共有鍵のローテーションだけで運用する案は、他利用者まで巻き込むため退ける。

## consequences
回答編集のcapability認可は維持するが、個別失効は後続作業として残る。
