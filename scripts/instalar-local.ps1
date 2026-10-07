# scripts/instalar-local.ps1 — instala o Zeca a partir de artefatos já prontos
# (sem cargo, sem o repo). É o miolo do instalador por download (M9, Windows): o
# scripts/get.ps1 baixa o pacote de release e chama este, que acha os artefatos
# ao lado dele:
#
#   <dir>\bichinho.exe                      o daemon
#   <dir>\skin\zeca-livre-escuro\           a skin padrão CC0 já aprovada
#   <dir>\.claude-plugin\marketplace.json   o marketplace do plugin
#   <dir>\plugin\                           o plugin do Claude Code
#
# Não-interativo.
$ErrorActionPreference = "Stop"

$DIR = Split-Path -Parent $MyInvocation.MyCommand.Path
$base = Join-Path $env:LOCALAPPDATA "bichinho"
$bindir = Join-Path $base "bin"
$exe = Join-Path $bindir "bichinho.exe"
$porta = 27380

function Passo($t) { Write-Host "▸ $t" }
function Info($t) { Write-Host "  $t" }

Write-Host "`n🦜 Instalando o Zeca…`n"

# 1. o binário -------------------------------------------------------------
Passo "instalando o bichinho.exe"
New-Item -ItemType Directory -Force -Path $bindir | Out-Null
Copy-Item -Force (Join-Path $DIR "bichinho.exe") $exe
Info (& $exe versao 2>&1 | Select-Object -First 1)

# 2. o hook no PATH do usuário (o hook do plugin acha `bichinho` por aqui) --
Passo "pondo o hook no PATH"
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($null -eq $userPath) { $userPath = "" }
if (($userPath -split ';') -notcontains $bindir) {
    [Environment]::SetEnvironmentVariable("Path", ($userPath.TrimEnd(';') + ";" + $bindir), "User")
    Info "adicionei $bindir ao PATH do usuário (abra um terminal novo para valer)"
}

# 3. config + a skin padrão JÁ APROVADA (snapshot em <estado>\skins\<id>) ---
Passo "config e a skin padrão (Zeca, CC0)"
$cfg = Join-Path $base "bichinho.toml"
if (-not (Test-Path $cfg)) {
    "[aparencia]`nskin = `"zeca-livre-escuro`"`ntamanho = `"pequeno`"`n" |
        Set-Content -NoNewline -Encoding ascii $cfg
}
$skinDst = Join-Path $base "skins\zeca-livre-escuro"
New-Item -ItemType Directory -Force -Path (Split-Path $skinDst) | Out-Null
if (Test-Path $skinDst) { Remove-Item -Recurse -Force $skinDst }
Copy-Item -Recurse -Force (Join-Path $DIR "skin\zeca-livre-escuro") $skinDst

# 4. sobe no login (chave Run) e agora, escondido (launcher .vbs, sem janela) -
Passo "subindo o Zeca no login (chave Run)"
$vbs = Join-Path $bindir "iniciar.vbs"
@"
Set sh = CreateObject("WScript.Shell")
sh.Environment("PROCESS")("PET_CONFIG") = "$base"
sh.Run """$exe"" rodar", 0, False
"@ | Set-Content -Encoding ascii $vbs
$run = "wscript.exe `"$vbs`""
New-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" `
    -Name "Zeca" -Value $run -PropertyType String -Force | Out-Null

# sobe agora
Start-Process wscript.exe -ArgumentList "`"$vbs`""
$subiu = $false
for ($i = 0; $i -lt 30; $i++) {
    try {
        Invoke-RestMethod -Uri "http://127.0.0.1:$porta/v1/estado" -Headers @{ "X-Pet" = "1" } -TimeoutSec 2 | Out-Null
        $subiu = $true; break
    } catch { Start-Sleep -Milliseconds 300 }
}
if ($subiu) { Info "o Zeca está no ar em 127.0.0.1:$porta" } else { Info "o Zeca ainda não respondeu (ele sobe de novo no próximo login)" }

# 5. pluga no Claude Code (fonte estável, não o tmp do download) -----------
if (Get-Command claude -ErrorAction SilentlyContinue) {
    Passo "plugando no Claude Code"
    $fonte = Join-Path $base "plugin-fonte"
    if (Test-Path $fonte) { Remove-Item -Recurse -Force $fonte }
    New-Item -ItemType Directory -Force -Path $fonte | Out-Null
    Copy-Item -Recurse -Force (Join-Path $DIR ".claude-plugin") (Join-Path $fonte ".claude-plugin")
    Copy-Item -Recurse -Force (Join-Path $DIR "plugin") (Join-Path $fonte "plugin")
    try { claude plugin marketplace add $fonte 2>$null | Out-Null } catch {}
    try { claude plugin install bichinho@bichinho-local 2>$null | Out-Null } catch {}
    $plugado = $true
} else {
    Info "o «claude» não está no PATH — pulei o plugin (o Zeca aparece, mas não reage às sessões)"
    $plugado = $false
}

Write-Host "`n✓ Zeca instalado e no ar."
if ($plugado) {
    Write-Host "  Abra um terminal novo e rode «claude» — ele já reage sozinho."
    Write-Host "  (Nas abas do Claude já abertas, um /reload-plugins liga o Zeca.)"
}
Write-Host "  Tirar: apague a chave Run «Zeca» e a pasta $base (e claude plugin uninstall bichinho).`n"
