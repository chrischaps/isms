#Requires -Version 7
<#
.SYNOPSIS
    Mint invite codes for the dev database and copy the first one to the clipboard.

.DESCRIPTION
    Runs `isms-server invite` against the play database (the one isms-dev.cmd started,
    isms_play3 by default). A first sign-in for an email needs one of these codes;
    after that the email alone is enough. Codes are single-use.

    Builds the server if its binary is missing. If the API window is running, the
    binary is already current: the launcher builds before it serves.

.EXAMPLE
    .\scripts\dev\New-IsmsInvite.ps1
    .\scripts\dev\New-IsmsInvite.ps1 -Count 3 -Database isms_play4
#>
[CmdletBinding()]
param(
    # How many codes to mint.
    [ValidateRange(1, 100)]
    [int] $Count = 1,

    # Play database name (isms-dev.cmd's argument).
    [string] $Database = 'isms_play3',

    # Postgres URL prefix; the database name is appended.
    [string] $DbPrefix = 'postgres://isms:isms@localhost:5433/',

    # Leave the clipboard alone.
    [switch] $NoClipboard
)

$ErrorActionPreference = 'Stop'
$repo = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$exe = Join-Path $repo 'target\debug\isms-server.exe'

if (-not (Test-Path $exe)) {
    Write-Host "No $exe yet; building..." -ForegroundColor DarkGray
    cargo build -p isms-server --manifest-path (Join-Path $repo 'Cargo.toml')
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
}

$env:DATABASE_URL = "$DbPrefix$Database"
$env:RUST_LOG = 'warn'

$codes = & $exe invite --count $Count 2>&1 | ForEach-Object { "$_" } | Where-Object { $_ -match '^[A-Za-z0-9]+-[A-Za-z0-9]+$' }
if ($LASTEXITCODE -ne 0 -or -not $codes) {
    throw "isms-server invite failed against $Database. Is the isms-dev-db container up?"
}

$codes | ForEach-Object { Write-Host $_ -ForegroundColor Cyan }

if (-not $NoClipboard) {
    Set-Clipboard -Value ($codes | Select-Object -First 1)
    Write-Host "First code copied to the clipboard. Sign in at http://localhost:5173 with an email and this code." -ForegroundColor DarkGray
}
