# monogram — licença e origem

A fonte **monogram**, de Vinícius Menézio (datagoblin), é de domínio público:
**Creative Commons Zero v1.0 Universal (CC0 1.0)**. O texto da licença está em
`CC0-1.0.txt`, ao lado (copiado de
https://creativecommons.org/publicdomain/zero/1.0/legalcode.txt), e os créditos
originais em `credits.txt`.

- Página: https://datagoblin.itch.io/monogram — "License: Creative Commons Zero
  v1.0 Universal".
- Baixada em 2026-10-04: `monogram.zip`, sha256
  `81d0540290f73d6317dc86e34978af80babf8e24129941cdf087b0fdfb75c287`.
- Daqui o repositório guarda só `bitmap/monogram-bitmap.json` (sha256
  `847e18d10ecd0fd4362ee0088a1bc56f9b203136b544d45db17426ff91913938`): cada
  glifo em 12 linhas de bits (o bit 0 é a coluna da esquerda; 5 colunas, e uns
  poucos acentos passam para a sexta e a sétima), avanço de 6.
- `cargo xtask fonte` assa esse JSON na tabela
  `crates/pet-core/src/fonte/glifos.rs`, que vai dentro do binário (decisão
  0052). Nada da fonte é baixado em tempo de execução.

CC0 não pede atribuição; o crédito está no `NOTICE.md` mesmo assim.
