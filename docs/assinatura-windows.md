# Assinatura do build do Windows (Authenticode via SignPath Foundation)

O `bichinho.exe` do release pode ser assinado de graça pelo **[SignPath
Foundation](https://signpath.org/)** (programa de code-signing para projetos
open source). Enquanto não estiver configurado, o release sai **sem assinatura**
e o Windows mostra o aviso do SmartScreen («Mais informações → Executar assim
mesmo»); o CI já está pronto e **liga sozinho** quando as variáveis abaixo
existirem.

Por que o SignPath, e não um certificado próprio: desde 2023 todo certificado
Authenticode (OV/EV) exige token de hardware ou HSM na nuvem — caro e difícil de
automatizar. O SignPath Foundation assina com o certificado deles, de graça, para
OSS, e integra com o GitHub Actions. (A outra opção barata seria o Azure Trusted
Signing, ~US$ 10/mês.)

## 1. Inscrever o projeto (ação do Renan, revisão humana)

1. Peça acesso ao programa OSS em <https://signpath.org/> (ou
   <https://about.signpath.io/product/open-source>).
2. O SignPath confere o projeto e a identidade do mantenedor à mão — isto leva
   alguns dias e **não dá para fazer do CI**. Pré-requisitos que o projeto já
   cumpre: licença open source (MIT), repositório público, build reproduzível no
   CI.

## 2. Configurar a organização no SignPath (depois de aprovado)

No painel do SignPath, dentro da organização que eles criarem:

1. **Project** — crie um projeto (ex.: `claude-pet`).
2. **Artifact configuration** — configure para assinar um PE do Windows
   (Authenticode) — o `bichinho.exe` dentro do artefato `bichinho-exe-unsigned`.
3. **Signing policy** — crie uma política (ex.: `release`) usando o certificado
   do Foundation.
4. **Trusted build system** — ligue ao GitHub Actions deste repositório e ao
   workflow `release.yml` (o SignPath valida a origem do build).
5. **CI user + API token** — crie um usuário de CI e gere um token.

Anote: o **organization id**, o **slug do projeto**, o **slug da política** e o
**token**.

## 3. Pôr os segredos no GitHub

Em **Settings → Secrets and variables → Actions**:

- **Variables** (não são segredas):
  - `SIGNPATH_ORG_ID` — o organization id
  - `SIGNPATH_PROJECT_SLUG` — o slug do projeto
  - `SIGNPATH_POLICY_SLUG` — o slug da política (ex.: `release`)
- **Secret**:
  - `SIGNPATH_API_TOKEN` — o token do usuário de CI

A existência de `SIGNPATH_ORG_ID` é o que **liga** o passo de assinatura no
`release.yml` (os passos têm `if: ${{ vars.SIGNPATH_ORG_ID != '' }}`).

## 4. Como o release assina (automático)

O job `windows` do `release.yml`, num tag `vX.Y.Z`:

1. compila o `bichinho.exe`;
2. **se** o SignPath estiver configurado: sobe o exe como artefato, submete ao
   SignPath (`signpath/github-action-submit-signing-request`), espera, e baixa o
   exe **assinado** por cima do mesmo arquivo;
3. empacota o `zeca-windows-x64.zip` (`win-empacotar.ps1 -SkipBuild`, já com o
   exe assinado) e sobe no release.

## 5. Depois que a assinatura estiver no ar

- Tire a nota do SmartScreen do `README.md` (a linha «isn't code-signed yet, so
  SmartScreen warns once»).
- Confira a assinatura num Windows: `Get-AuthenticodeSignature bichinho.exe` deve
  dar `Valid`.
- A reputação do SmartScreen ainda cresce com o número de instalações, mesmo
  assinado; o aviso some mais rápido com a assinatura.

Referência: <https://docs.signpath.io/trusted-build-systems/github>
