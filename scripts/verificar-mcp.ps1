# Verifica se o webserver do TouchDesigner MCP está respondendo na porta 9981.
$ErrorActionPreference = 'Continue'
Write-Host '== TouchDesigner MCP check ==' -ForegroundColor Cyan

$tdPaths = @(
  "${env:ProgramFiles}\Derivative\*",
  "${env:ProgramFiles}\TouchDesigner*",
  "${env:LOCALAPPDATA}\Programs\Derivative\*"
)
$found = Get-ChildItem $tdPaths -ErrorAction SilentlyContinue | Where-Object {
  $_.Name -match 'TouchDesigner|Derivative'
}
if ($found) {
  Write-Host 'TouchDesigner encontrado:' -ForegroundColor Green
  $found | ForEach-Object { Write-Host "  $($_.FullName)" }
} else {
  Write-Host 'TouchDesigner NAO encontrado em Program Files.' -ForegroundColor Yellow
  Write-Host 'Baixe em https://derivative.ca/download (conta gratis Non-Commercial).'
}

$tox = Join-Path $PSScriptRoot '..\touchdesigner-mcp\mcp_webserver_base.tox'
if (Test-Path $tox) {
  Write-Host "Componente MCP OK: $tox" -ForegroundColor Green
} else {
  Write-Host "Componente MCP AUSENTE: $tox" -ForegroundColor Red
}

Write-Host 'Testando porta 9981...'
$tnc = Test-NetConnection 127.0.0.1 -Port 9981 -WarningAction SilentlyContinue
if ($tnc.TcpTestSucceeded) {
  Write-Host 'Porta 9981 ABERTA - webserver do TD esta no ar.' -ForegroundColor Green
  try {
    $r = Invoke-WebRequest -Uri 'http://127.0.0.1:9981/' -UseBasicParsing -TimeoutSec 3
    Write-Host "HTTP $($r.StatusCode) (corpo $($r.Content.Length) bytes)"
  } catch {
    Write-Host "Porta aberta, mas HTTP falhou: $($_.Exception.Message)" -ForegroundColor Yellow
  }
} else {
  Write-Host 'Porta 9981 FECHADA.' -ForegroundColor Yellow
  Write-Host 'Abra o TouchDesigner, carregue o Spectral Audio Reactive.toe,'
  Write-Host 'importe o mcp_webserver_base.tox e confirme no Textport (Alt+T).'
}
