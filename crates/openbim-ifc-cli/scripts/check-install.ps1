# Prove install.ps1 installs what the release ships on Windows, and refuses
# what it must (#378). Runs on Windows in .github/workflows/cli.yml and in
# each release build (release.yml, cli-binaries).
#
#   pwsh scripts/check-install.ps1 [-Binary PATH]
#
# Packs PATH (default: a fresh debug build) with scripts/package.py under
# the release name install.ps1 asks for on this machine, lays it out as a
# release directory with its SHA256SUMS, and serves it to install.ps1 as a
# file:// mirror (OPENBIM_IFC_BASE_URL). Then:
#   1. install into a prefix, with PowerShell 7 and with Windows PowerShell
#      5.1, as a script and the `irm | iex` way; the installed binary
#      reports the version;
#   2. a corrupted SHA256SUMS is refused and installs nothing;
#   3. a version with no release is refused;
#   4. -AddToPath adds the prefix to the user PATH once (restored after).
param([string] $Binary)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$crate = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$root = (Resolve-Path (Join-Path $crate '..\..')).Path
$installer = Join-Path $crate 'install.ps1'
Set-Location $root

function Fail([string] $Message) { throw "check-install.ps1: $Message" }
$python = if (Get-Command python -ErrorAction SilentlyContinue) { 'python' } else { 'python3' }
$targetDir = (& cargo metadata --format-version 1 --no-deps | ConvertFrom-Json).target_directory

if (-not $Binary) {
    & cargo build --quiet -p openbim-ifc-cli
    if ($LASTEXITCODE -ne 0) { Fail 'cargo build failed' }
    $Binary = Join-Path $targetDir 'debug\openbim-ifc.exe'
}
$Binary = (Resolve-Path $Binary).Path
$version = (Select-String -Path (Join-Path $crate 'Cargo.toml') -Pattern '^version = "(.*)"$' |
    Select-Object -First 1).Matches[0].Groups[1].Value
$target = & pwsh -NoProfile -File $installer -PrintTarget
if ($LASTEXITCODE -ne 0) { Fail 'install.ps1 -PrintTarget failed' }

# Scratch space inside the build directory.
New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
$work = Join-Path $targetDir ('check-install-ps1.' + [Guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $work | Out-Null
$savedUserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$savedBase = $env:OPENBIM_IFC_BASE_URL
try {
    $releases = Join-Path $work 'releases'
    $release = Join-Path $releases "openbim-ifc-cli-v$version"
    & $python (Join-Path $crate 'scripts\package.py') archive --target $target --binary $Binary --out $release | Out-Null
    if ($LASTEXITCODE -ne 0) { Fail 'package.py archive failed' }
    & $python (Join-Path $crate 'scripts\package.py') checksums $release --expect 1 | Out-Null
    if ($LASTEXITCODE -ne 0) { Fail 'package.py checksums failed' }
    $env:OPENBIM_IFC_BASE_URL = ([Uri] $releases).AbsoluteUri

    function Invoke-Installer([string] $Shell, [string[]] $Arguments) {
        # Out-Host: only the exit code is this function's output.
        & $Shell -NoProfile -ExecutionPolicy Bypass -File $installer @Arguments | Out-Host
        return $LASTEXITCODE
    }
    function Assert-Installed([string] $Prefix) {
        $exe = Join-Path $Prefix 'bin\openbim-ifc.exe'
        if (-not (Test-Path -LiteralPath $exe)) { Fail "nothing installed at $exe" }
        $reported = & $exe --version
        if ($reported -ne "openbim-ifc $version") { Fail "installed binary says '$reported'" }
    }

    # 1. A good install, from both PowerShells, as a file and through iex.
    $code = Invoke-Installer 'pwsh' @('-Version', $version, '-Prefix', (Join-Path $work 'good'), '-NoModifyPath')
    if ($code -ne 0) { Fail "install.ps1 exited $code" }
    Assert-Installed (Join-Path $work 'good')

    if (Get-Command powershell.exe -ErrorAction SilentlyContinue) {
        $code = Invoke-Installer 'powershell.exe' @('-Version', "openbim-ifc-cli-v$version", '-Prefix', (Join-Path $work 'desktop'), '-NoModifyPath')
        if ($code -ne 0) { Fail "install.ps1 under Windows PowerShell exited $code" }
        Assert-Installed (Join-Path $work 'desktop')
    } else {
        Write-Warning 'no Windows PowerShell 5.1 here; checked PowerShell 7 only'
    }

    # `irm ... | iex` takes its options from the environment.
    $env:OPENBIM_IFC_VERSION = $version
    $env:OPENBIM_IFC_PREFIX = Join-Path $work 'iex'
    $env:OPENBIM_IFC_NO_MODIFY_PATH = '1'
    try {
        & pwsh -NoProfile -Command "Get-Content -Raw -LiteralPath '$installer' | Invoke-Expression"
        if ($LASTEXITCODE -ne 0) { Fail "install.ps1 through Invoke-Expression exited $LASTEXITCODE" }
    } finally {
        Remove-Item Env:OPENBIM_IFC_VERSION, Env:OPENBIM_IFC_PREFIX, Env:OPENBIM_IFC_NO_MODIFY_PATH
    }
    Assert-Installed (Join-Path $work 'iex')

    # 2. A checksum mismatch installs nothing.
    $sums = Join-Path $release 'SHA256SUMS'
    $good = Get-Content -Raw -LiteralPath $sums
    $digit = $good.Substring(0, 1)
    $corrupt = $(if ($digit -eq '0') { 'f' } else { '0' }) + $good.Substring(1)
    Set-Content -LiteralPath $sums -Value $corrupt -NoNewline
    $log = Join-Path $work 'bad.log'
    & pwsh -NoProfile -File $installer -Version $version -Prefix (Join-Path $work 'bad') -NoModifyPath 2> $log
    if ($LASTEXITCODE -eq 0) { Fail 'a corrupted SHA256SUMS was accepted' }
    if (-not (Select-String -LiteralPath $log -Pattern 'checksum mismatch' -Quiet)) {
        Fail "unexpected refusal: $(Get-Content -Raw -LiteralPath $log)"
    }
    if (Test-Path -LiteralPath (Join-Path $work 'bad\bin\openbim-ifc.exe')) { Fail 'a refused download was installed' }
    Set-Content -LiteralPath $sums -Value $good -NoNewline

    # 3. A version with no release.
    & pwsh -NoProfile -File $installer -Version '0.0.0-none' -Prefix (Join-Path $work 'none') -NoModifyPath 2> $null
    if ($LASTEXITCODE -eq 0) { Fail 'a missing release was accepted' }
    if (Test-Path -LiteralPath (Join-Path $work 'none\bin\openbim-ifc.exe')) { Fail 'a missing release installed something' }

    # 4. -AddToPath, once, however often it runs.
    $pathPrefix = Join-Path $work 'path'
    foreach ($round in 1, 2) {
        $code = Invoke-Installer 'pwsh' @('-Version', $version, '-Prefix', $pathPrefix, '-AddToPath')
        if ($code -ne 0) { Fail "install.ps1 -AddToPath exited $code (round $round)" }
    }
    $entry = (Resolve-Path (Join-Path $pathPrefix 'bin')).Path
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $count = @($userPath -split ';' | Where-Object { $_ -ieq $entry }).Count
    if ($count -ne 1) { Fail "the user PATH holds $entry $count times: $userPath" }
} finally {
    [Environment]::SetEnvironmentVariable('Path', $savedUserPath, 'User')
    $env:OPENBIM_IFC_BASE_URL = $savedBase
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Output "check-install.ps1: ok ($target, openbim-ifc $version)"
