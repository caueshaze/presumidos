param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Username,

    [Parameter(Mandatory = $true, Position = 1)]
    [string]$Email
)

$ErrorActionPreference = "Stop"

Set-Location (Join-Path $PSScriptRoot "..")

$cargoArgs = @(
    "run",
    "-p", "ferrugem-web",
    "--",
    "bootstrap-admin",
    "--username", $Username,
    "--email", $Email
)

if ($env:BOOTSTRAP_ADMIN_PASSWORD) {
    $env:BOOTSTRAP_ADMIN_PASSWORD = $env:BOOTSTRAP_ADMIN_PASSWORD.Trim()
}

& cargo @cargoArgs
