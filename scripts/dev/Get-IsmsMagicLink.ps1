#Requires -Version 7
<#
.SYNOPSIS
    Fetch the newest magic (sign-in) link the dev server sent, and copy it to the clipboard.

.DESCRIPTION
    The server keeps only a hash of each link's token, so a link cannot be read back
    from Postgres: it has to be caught as it is sent. With ISMS_MAIL_SENDER=file the
    server appends `email<TAB>url` to ISMS_MAIL_FILE for every link (isms-dev.cmd sets
    this up: %LOCALAPPDATA%\isms\mail-<database>.log). This script reads that file.

    Flow: mint a code with New-IsmsInvite.ps1, the player enters email + code at
    http://localhost:5173, then run this to pick up the link they were "sent".
    Links are single-use and expire after 15 minutes.

.EXAMPLE
    .\scripts\dev\Get-IsmsMagicLink.ps1                      # newest link, any email
    .\scripts\dev\Get-IsmsMagicLink.ps1 -Email friend@x.org  # newest for that email
    .\scripts\dev\Get-IsmsMagicLink.ps1 -Wait -Open          # wait for the next one, open it
#>
[CmdletBinding()]
param(
    # Only links sent to this address (case-insensitive). Default: any.
    [string] $Email,

    # Play database name; picks the mailbox file isms-dev.cmd writes for it.
    [string] $Database = 'isms_play3',

    # Mailbox file, if not the launcher's default.
    [string] $Path,

    # Block until a link newer than the ones already in the file arrives.
    [switch] $Wait,

    # Seconds to wait before giving up (with -Wait).
    [int] $TimeoutSeconds = 900,

    # Open the link in the default browser (signs *this* browser in).
    [switch] $Open,

    # Leave the clipboard alone.
    [switch] $NoClipboard
)

$ErrorActionPreference = 'Stop'
if (-not $Path) { $Path = Join-Path $env:LOCALAPPDATA "isms\mail-$Database.log" }

function Read-Links {
    if (-not (Test-Path $Path)) { return @() }
    Get-Content $Path | ForEach-Object {
        $to, $url = $_ -split "`t", 2
        if ($url) { [pscustomobject]@{ To = $to; Url = $url } }
    } | Where-Object { -not $Email -or $_.To -ieq $Email }
}

$links = @(Read-Links)

if ($Wait) {
    $seen = $links.Count
    $who = if ($Email) { $Email } else { 'anyone' }
    Write-Host "Waiting for a sign-in link for $who ($Path)..." -ForegroundColor DarkGray
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ($true) {
        $links = @(Read-Links)
        if ($links.Count -gt $seen) { break }
        if ((Get-Date) -gt $deadline) { throw "No new link within $TimeoutSeconds s." }
        Start-Sleep -Seconds 1
    }
}

$link = $links | Select-Object -Last 1
if (-not $link) {
    $hint = if (Test-Path $Path) { "no link for $Email in $Path" } else { "$Path does not exist. Is the API window running with ISMS_MAIL_SENDER=file? (isms-dev.cmd sets it)" }
    throw "No sign-in link found: $hint"
}

Write-Host "To:   $($link.To)"
Write-Host "Link: " -NoNewline
Write-Host $link.Url -ForegroundColor Cyan

if (-not $NoClipboard) {
    Set-Clipboard -Value $link.Url
    Write-Host "Copied to the clipboard. Single-use; 15 minutes." -ForegroundColor DarkGray
}
if ($Open) { Start-Process $link.Url }

$link
