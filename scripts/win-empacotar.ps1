# scripts/win-empacotar.ps1 <staging> — monta o pacote do Zeca para Windows (M9):
# o bichinho.exe, a skin CC0 padrão JÁ APROVADA (snapshot + aprovacao.json), o
# plugin e o instalar-local.ps1. Usado pelo release e pelo teste do instalador.
# Compila o daemon antes (release). Sem assinatura: o SmartScreen avisa uma vez.
#
#   scripts/win-empacotar.ps1 -Staging staging
param([Parameter(Mandatory = $true)][string]$Staging)
$ErrorActionPreference = "Stop"

$raiz = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$cargo = if ($env:CARGO) { $env:CARGO } else { "cargo" }

Write-Host "- compilando o daemon (release)"
& $cargo build --release -p bichinho
if ($LASTEXITCODE -ne 0) { throw "cargo build falhou" }

if (Test-Path $Staging) { Remove-Item -Recurse -Force $Staging }
New-Item -ItemType Directory -Force -Path $Staging | Out-Null

Write-Host "- montando o pacote"
Copy-Item -Force (Join-Path $raiz "target\release\bichinho.exe") (Join-Path $Staging "bichinho.exe")

# A skin padrão (CC0), JA APROVADA. A impressão digital é o sha256 do texto
# "sha256(arquivo)  nome\n" de skin.json, sheet.json, sheet.png — exatamente o
# que o daemon calcula (pet_core::aprovacao::impressao).
$skinSrc = Join-Path $raiz "skins\zeca-livre-escuro"
$skinDst = Join-Path $Staging "skin\zeca-livre-escuro"
New-Item -ItemType Directory -Force -Path $skinDst | Out-Null
Copy-Item -Recurse -Force (Join-Path $skinSrc "*") $skinDst

function Impressao($dir) {
    $lista = ""
    foreach ($f in @("skin.json", "sheet.json", "sheet.png")) {
        $h = (Get-FileHash -Algorithm SHA256 (Join-Path $dir $f)).Hash.ToLower()
        $lista += "$h  $f`n"
    }
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($lista)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString("x2") }) -join ""
}

$fp = Impressao $skinDst
$ms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
"{`"id`":`"zeca-livre-escuro`",`"sha256`":`"$fp`",`"aprovada_em_ms`":$ms}" |
    Set-Content -NoNewline -Encoding ascii (Join-Path $skinDst "aprovacao.json")
Write-Host "  impressão da skin: $fp"

Copy-Item -Recurse -Force (Join-Path $raiz "plugin") (Join-Path $Staging "plugin")
Copy-Item -Recurse -Force (Join-Path $raiz ".claude-plugin") (Join-Path $Staging ".claude-plugin")
Copy-Item -Force (Join-Path $raiz "scripts\instalar-local.ps1") (Join-Path $Staging "instalar-local.ps1")

Write-Host "✓ pacote em $Staging"
