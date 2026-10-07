# scripts/get.ps1 — instale o Zeca com UM comando no PowerShell, sem o repo e
# sem cargo:
#
#   irm https://raw.githubusercontent.com/butkeraites/claude-pet/main/scripts/get.ps1 | iex
#
# Baixa o pacote de release do GitHub (binário já compilado) e roda o setup
# local. Variáveis opcionais:
#   $env:ZECA_VERSION = "vX.Y.Z"   fixa a versão (padrão: a última release)
#   $env:ZECA_REPO    = "dono/repo"  outro repositório
$ErrorActionPreference = "Stop"

$repo = if ($env:ZECA_REPO) { $env:ZECA_REPO } else { "butkeraites/claude-pet" }
$asset = "zeca-windows-x64.zip"
$ver = $env:ZECA_VERSION
$url = if ($ver) {
    "https://github.com/$repo/releases/download/$ver/$asset"
} else {
    "https://github.com/$repo/releases/latest/download/$asset"
}

$tmp = Join-Path $env:TEMP ("zeca-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
try {
    Write-Host "🦜 Baixando o Zeca (windows/x64)…"
    $zip = Join-Path $tmp "zeca.zip"
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    } catch {}
    Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
    Expand-Archive -Path $zip -DestinationPath $tmp -Force
    & (Join-Path $tmp "instalar-local.ps1")
} catch {
    Write-Host "falhou baixar ou instalar: $_" -ForegroundColor Red
    Write-Host "  (há um release com $asset? veja https://github.com/$repo/releases)"
    throw
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
