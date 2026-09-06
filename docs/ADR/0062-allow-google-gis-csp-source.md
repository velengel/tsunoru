# ADR 0062: Google GISの公式スクリプトURLだけをCSPで許可する

## context

主催者ログインはGoogle Identity Servicesのブラウザライブラリを必要とする。一方、Workerは`script-src 'self'`を含むCSPを返しており、外部スクリプトは既定で拒否される。Google公式はGISライブラリを`https://accounts.google.com/gsi/client`から読み込み、CSPの`script-src`にも同じURLを追加する構成を案内している。

## decision

CSPの`script-src`に`https://accounts.google.com/gsi/client`だけを追加し、GISの`connect-src`通信は`https://accounts.google.com`に限定する。

## rejected options

- CSPを変更しない: GISライブラリがブラウザーでブロックされ、主催者ログインが動かない。
- `https://accounts.google.com`全体を`script-src`へ追加する: 必要以上に広い許可範囲になる。
- GISを自前ホスティングする: Google公式の非対応構成で、セキュリティ更新を受けられない。
- Report-Onlyだけにする:検証には使えるが、実運用のCSP保護にならない。

## consequences

Googleが提供するGIS外部スクリプトに依存する。許可範囲は公式の単一スクリプトURLに限定できる。GISが追加で要求するiframeや通信先が将来発生した場合は、公式ドキュメントを確認して個別にCSPを更新する。
