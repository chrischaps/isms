#Requires -Version 7
<#
.SYNOPSIS
    A lively Freeport to play in: a town of Jev players with room kept for you and a friend.

.DESCRIPTION
    Its own database (isms_lively) and its own ports (API 8090, web 5174), so it runs beside
    isms-dev.cmd without touching isms_play3. On a first start it seeds a lab Freeport the way
    the SJ.6b week was seeded (sixteen players from the freeport-town cast, eight householders,
    E-4's places kept empty) plus a place for each human, at a human pace (30 s an hour: a day
    in 12 minutes, the seven-day week in about 85).

    It opens three windows: the API, the web client, and the Jev cohort. The clock is held from
    the moment the server is up until you press Enter here, so the two of you can sign in and
    join first. The cohort plays to the epoch's end and writes docs/playtest/runs/<run>.md.

    Jev spends OpenRouter credit (OPENROUTER_API_KEY from the repo's .env): the SJ.6b week of
    sixteen players cost $0.15. The cost follows game hours, not wall time, and -MaxUsd caps it.

    Run it again to pick the same town back up (the cohort resumes from its accounts.json).
    Stop with -Stop, which holds the clock first: a server that was closed while its clock ran
    plays every missed hour back to back on the next start.

.EXAMPLE
    .\scripts\dev\Start-IsmsLively.ps1                  # seed (first time) or resume, hold, start on Enter
    .\scripts\dev\Start-IsmsLively.ps1 -Stop            # hold the clock and close the three windows
    .\scripts\dev\Start-IsmsLively.ps1 -Reseed          # throw the town away and seed a fresh one
    .\scripts\dev\Start-IsmsLively.ps1 -Reseed -TickSeconds 60 -Days 3
#>
[CmdletBinding()]
param(
    # Seconds of wall time per game hour. Change it live on the Operator page (/admin).
    [ValidateRange(1, 3600)] [int] $TickSeconds = 30,
    # Days in the epoch.
    [ValidateRange(1, 42)] [int] $Days = 7,
    # Humans the seed keeps a place for (you and a friend).
    [ValidateRange(0, 16)] [int] $Humans = 2,
    # Synthetic players, cycled through the cast's persona list (sixteen is the whole town).
    [ValidateRange(1, 32)] [int] $Players = 16,
    # A named cast from agents/config.toml `run.personas_for`.
    [string] $Cast = 'freeport-town',
    # Householders beside the players (SJ.6b's town had eight).
    [ValidateRange(0, 64)] [int] $Householders = 8,
    # Minutes the closing statements stay open after the epoch ends.
    [int] $ClosingMinutes = 30,
    # jev (the town's personas on Jev, the rule-prober scripted) or scripted (free; for checking the launcher).
    [ValidateSet('jev', 'scripted')] [string] $Brain = 'jev',
    # The cohort stops cleanly at this spend.
    [double] $MaxUsd = 1,
    # The operator (pause, step, tick length on /admin).
    [string] $Operator = 'chrischappelear@gmail.com',
    [string] $Database = 'isms_lively',
    [int] $ApiPort = 8090,
    [int] $WebPort = 5174,
    # Drop the database and seed a new town.
    [switch] $Reseed,
    # Start the clock at once instead of waiting for Enter.
    [switch] $NoHold,
    # Hold the clock and close the windows.
    [switch] $Stop
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$stateDir = Join-Path $env:LOCALAPPDATA 'isms'
$stateFile = Join-Path $stateDir "lively-$Database.json"
$mailFile = Join-Path $stateDir "mail-$Database.log"
# The server runs from a copy, so a build (run.sh, make push) never meets a locked exe.
$exe = Join-Path $repo 'target\lively\isms-server.exe'
$api = "http://127.0.0.1:$ApiPort"
$web = "http://localhost:$WebPort"
$pg = 'isms-dev-db'
$env:DATABASE_URL = "postgres://isms:isms@localhost:5433/$Database"
New-Item -ItemType Directory -Force $stateDir | Out-Null

function Say([string] $text) { Write-Host "`n== $text" -ForegroundColor Cyan }

function Read-State {
    if (Test-Path $stateFile) { Get-Content $stateFile -Raw | ConvertFrom-Json } else { $null }
}

function Write-State($state) { $state | ConvertTo-Json | Set-Content $stateFile }

# An operator session minted straight from the database, for the /admin calls.
function Invoke-Admin([int] $sid, [string] $verb) {
    $env:RUST_LOG = 'warn'
    $token = & $exe session --email $Operator | Select-Object -Last 1
    if ($LASTEXITCODE -ne 0 -or -not $token) { throw "isms-server session failed" }
    $headers = @{ Cookie = "isms_session=$token"; 'X-Requested-With' = 'isms' }
    Invoke-RestMethod -Method Post "$api/admin/s/$sid/$verb" -Headers $headers | Out-Null
}

# A window running one command, inheriting this process's environment. -EncodedCommand keeps
# every path and quote intact (nested quoting in Start-Process arguments is what broke isms-dev.cmd once).
function Start-Window([string] $title, [string] $command) {
    $script = "`$Host.UI.RawUI.WindowTitle = '$title'; Set-Location '$repo'; $command"
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($script))
    (Start-Process pwsh -ArgumentList '-NoExit', '-NoProfile', '-EncodedCommand', $encoded -PassThru).Id
}

function Test-Listening([int] $port) {
    [bool](Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue)
}

# -- stop ---------------------------------------------------------------------

if ($Stop) {
    $state = Read-State
    if (-not $state) { throw "No lively town recorded in $stateFile." }
    if (Test-Listening $ApiPort) {
        Invoke-Admin $state.society 'pause'
        Write-Host "Clock held for society $($state.society)."
    }
    foreach ($id in @($state.windows)) {
        if ($id) { taskkill /T /F /PID $id 2>&1 | Out-Null }
    }
    Write-Host "Windows closed. Run the script again to pick the town back up."
    return
}

foreach ($port in $ApiPort, $WebPort) {
    if (Test-Listening $port) { throw "Port $port is already in use. Is a lively town running? Stop it with -Stop." }
}

# -- database -----------------------------------------------------------------

Say "Postgres ($pg)"
docker start $pg | Out-Null
if ($LASTEXITCODE -ne 0) { throw "Could not start the $pg container. Is Docker Desktop running?" }
$tries = 0
while ($true) {
    docker exec $pg pg_isready -q 2>&1 | Out-Null
    if ($LASTEXITCODE -eq 0) { break }
    if (++$tries -ge 30) { throw "$pg did not accept connections within 30 s (docker logs $pg)." }
    Start-Sleep -Seconds 1
}
$exists = (docker exec $pg psql -U isms -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname = '$Database'") -eq '1'

$state = Read-State
if ($Reseed) {
    if ($exists) {
        Say "drop $Database"
        docker exec $pg dropdb -U isms $Database
        if ($LASTEXITCODE -ne 0) { throw "dropdb $Database failed" }
    }
    Remove-Item $stateFile, $mailFile -ErrorAction SilentlyContinue
    $state = $null
} elseif (-not $state -and $exists) {
    throw "$Database exists but $stateFile does not; pass -Reseed to start a fresh town in it."
}

# -- build --------------------------------------------------------------------

Say "build the server and the harness"
# run.sh's target directory: warm, and not the dev stack's (its running exe locks target\debug).
$env:CARGO_TARGET_DIR = Join-Path $repo 'target\check'
$env:SQLX_OFFLINE = 'true'
cargo build -q -p isms-server --manifest-path (Join-Path $repo 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
Remove-Item Env:CARGO_TARGET_DIR, Env:SQLX_OFFLINE
New-Item -ItemType Directory -Force (Split-Path $exe) | Out-Null
Copy-Item (Join-Path $repo 'target\check\debug\isms-server.exe') $exe -Force
pnpm --dir (Join-Path $repo 'agents') install --frozen-lockfile --silent
if ($LASTEXITCODE -ne 0) { throw "pnpm install (agents) failed" }

# -- seed ---------------------------------------------------------------------

$env:RUST_LOG = 'warn'
Push-Location $repo
try {
    if (-not $state) {
        $expect = $Players + $Humans
        $floor = $expect + $Householders
        $run = 'lively-' + (Get-Date -Format 'yyyyMMdd-HHmm')
        Say "seed a lab Freeport: $Players Jev town players, $Humans places for humans, $Householders householders, $Days days at $TickSeconds s an hour"
        & $exe migrate
        if ($LASTEXITCODE -ne 0) { throw "migrate failed" }
        # Mirrors scripts/agents/run.sh: the players' places left empty (E-4), a dwelling for everyone
        # in the floor and one more per player (Q159), no collapse in a small town.
        $sid = & $exe seed --preset freeport --class lab --name $run --tick-seconds $TickSeconds `
            --expect-humans $expect `
            --param "params.time.epoch_cycles=$Days" `
            --param "params.time.closing_window_minutes=$ClosingMinutes" `
            --param "params.initial_dwellings=$($floor + $expect)" `
            --param "params.population.floor=$floor" `
            --param params.population.collapse_enabled=false | Select-Object -Last 1
        if ($LASTEXITCODE -ne 0 -or $sid -notmatch '^\d+$') { throw "seed failed ($sid)" }
        $state = [pscustomobject]@{ society = [int]$sid; run = $run; players = $Players; cast = $Cast; windows = @() }
        Write-State $state
    } else {
        Say "resume society $($state.society) (run $($state.run))"
        & $exe migrate
        if ($LASTEXITCODE -ne 0) { throw "migrate failed" }
    }
} finally {
    Pop-Location
}
$sid = $state.society

# -- windows ------------------------------------------------------------------

Say "start the API on $api and the web client on $web"
$env:ISMS_OPERATORS = $Operator
$env:ISMS_MAIL_SENDER = 'file'
$env:ISMS_MAIL_FILE = $mailFile
$env:RUST_LOG = 'info'
Remove-Item Env:ISMS_TICK_SECONDS, Env:ISMS_BIND, Env:ISMS_BASE_URL -ErrorAction SilentlyContinue
$apiWindow = Start-Window "Isms lively API ($Database)" "& '$exe' serve --bind 127.0.0.1:$ApiPort --base-url $web"
$env:ISMS_API = $api
$webWindow = Start-Window 'Isms lively web' "pnpm --dir web dev --port $WebPort --strictPort"

$deadline = (Get-Date).AddSeconds(90)
while ($true) {
    try { Invoke-WebRequest "$api/healthz" -TimeoutSec 2 | Out-Null; break } catch {}
    if ((Get-Date) -gt $deadline) { throw "The API did not answer $api/healthz within 90 s; look at its window." }
    Start-Sleep -Milliseconds 500
}
# A town stopped with -Stop comes back held; -NoHold releases it.
Invoke-Admin $sid $(if ($NoHold) { 'resume' } else { 'pause' })

# The cohort: the harness joins its players (or finds them in accounts.json) and plays each hour.
$key = Get-Content (Join-Path $repo '.env') -ErrorAction SilentlyContinue |
    Where-Object { $_ -match '^\s*OPENROUTER_API_KEY\s*=' } | ForEach-Object { ($_ -split '=', 2)[1].Trim().Trim('"') } |
    Select-Object -First 1
if ($Brain -eq 'jev' -and -not $key) { throw "OPENROUTER_API_KEY is not in $repo\.env; the Jev players need it." }
if ($key) { $env:OPENROUTER_API_KEY = $key }
$env:ISMS_URL = $api
$env:ISMS_SOCIETY = "$sid"
$env:ISMS_SERVER_BIN = $exe
$env:RUST_LOG = 'warn'
$cohort = "pnpm --dir agents play --run $($state.run) --preset freeport --players $($state.players) --brain $Brain --cast $($state.cast) --max-usd $MaxUsd"
$cohortWindow = Start-Window "Isms lively Jev town ($($state.run))" $cohort
$state.windows = @($apiWindow, $webWindow, $cohortWindow)
Write-State $state

# -- sign in ------------------------------------------------------------------

$env:RUST_LOG = 'warn'
$codes = & $exe invite --count $Humans | Where-Object { $_ -match '^[A-Za-z0-9]+-[A-Za-z0-9]+$' }

Write-Host ""
Write-Host "Lively Freeport is up: society $sid, run $($state.run)." -ForegroundColor Green
Write-Host "  Play      $web          (the lab town is in your society list; pick a handle and join)"
Write-Host "  Operator  $web/admin    (pause, step, tick length)"
Write-Host "  Invites   $($codes -join '  ')   (a first sign-in for an email needs one; yours works without)"
Write-Host "  Links     .\scripts\dev\Get-IsmsMagicLink.ps1 -Database $Database -Wait"
Write-Host "            Your friend signs in from a second browser profile or an InPrivate window, so the two sessions stay apart."
Write-Host "  Stop      .\scripts\dev\Start-IsmsLively.ps1 -Stop"
if ($NoHold) { return }

Write-Host ""
Read-Host "The clock is held. When you have both joined, press Enter to start it"
Invoke-Admin $sid 'resume'
Write-Host "Clock running. The town takes its first turns on the next hour." -ForegroundColor Green
