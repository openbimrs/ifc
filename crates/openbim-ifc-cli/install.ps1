# Install the openbim-ifc command on Windows from a GitHub release of
# openbimrs/ifc (#378).
#
#   irm https://raw.githubusercontent.com/openbimrs/ifc/main/crates/openbim-ifc-cli/install.ps1 | iex
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/openbimrs/ifc/main/crates/openbim-ifc-cli/install.ps1))) -Version 0.1.0 -Prefix C:\Tools\openbim-ifc
#
# It picks the archive for this CPU (x86_64 or ARM64), downloads it with the
# release's SHA256SUMS, refuses to install unless the checksum matches
# (Get-FileHash), and copies openbim-ifc.exe to PREFIX\bin (default
# %LOCALAPPDATA%\Programs\openbim-ifc\bin). When that directory is not on
# the user PATH, it asks whether to add it (in an interactive session),
# adds it with -AddToPath, or leaves PATH alone with -NoModifyPath.
#
# Parameters (or the environment variable in brackets, for `irm | iex`):
#   -Version V     a release, e.g. 0.1.0; default the newest  [OPENBIM_IFC_VERSION]
#   -Prefix DIR    install into DIR\bin                       [OPENBIM_IFC_PREFIX]
#   -AddToPath     add PREFIX\bin to the user PATH without asking
#   -NoModifyPath  never change the user PATH                 [OPENBIM_IFC_NO_MODIFY_PATH=1]
#   -PrintTarget   print the release target for this machine and exit
# Mirrors and tests may point the downloads elsewhere: OPENBIM_IFC_BASE_URL
# (default https://github.com/openbimrs/ifc/releases/download), under which
# each release is a directory named after its tag; file:// URLs work too.
# Runs on Windows PowerShell 5.1 and PowerShell 7.
param(
    [string] $Version = $env:OPENBIM_IFC_VERSION,
    [string] $Prefix = $env:OPENBIM_IFC_PREFIX,
    [switch] $AddToPath,
    [switch] $NoModifyPath,
    [switch] $PrintTarget
)

# A child scope, so that `irm | iex` leaves the caller's preferences,
# strict mode and functions as they were.
& {
    Set-StrictMode -Version 3.0
    $ErrorActionPreference = 'Stop'
    # Invoke-WebRequest's progress bar slows downloads tenfold on 5.1.
    $ProgressPreference = 'SilentlyContinue'

    $Repo = 'openbimrs/ifc'
    $Crate = 'openbim-ifc-cli'
    $Bin = 'openbim-ifc'

    function Say([string] $Message) { [Console]::Error.WriteLine("install.ps1: $Message") }
    # throw, never exit: under `irm | iex` exit would close the caller's shell.
    function Fail([string] $Message) { throw "install.ps1: error: $Message" }

    # The release target for this machine: the names release.yml builds.
    function Get-Target {
        if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
            Fail "install.ps1 is for Windows; on Linux and macOS use install.sh"
        }
        # The native CPU, also from an x64 PowerShell emulated on ARM64:
        # PROCESSOR_ARCHITECTURE of the process says AMD64 there, the machine's
        # environment says ARM64.
        $arch = $null
        try {
            $arch = (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment' -Name PROCESSOR_ARCHITECTURE).PROCESSOR_ARCHITECTURE
        } catch {
            $arch = $null
        }
        if (-not $arch) {
            $arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
        }
        switch ($arch.ToUpperInvariant()) {
            'AMD64' { return 'x86_64-pc-windows-msvc' }
            'ARM64' { return 'aarch64-pc-windows-msvc' }
            default { Fail "no prebuilt binary for CPU $arch; build it with: cargo install $Crate" }
        }
    }

    function Get-Download([string] $Url, [string] $Destination) {
        $uri = [Uri] $Url
        if ($uri.IsFile) {
            # Invoke-WebRequest on PowerShell 7 refuses file://; a mirror on
            # disk is a copy.
            if (-not (Test-Path -LiteralPath $uri.LocalPath -PathType Leaf)) {
                throw "no file at $($uri.LocalPath)"
            }
            Copy-Item -LiteralPath $uri.LocalPath -Destination $Destination
        } else {
            Invoke-WebRequest -Uri $uri -OutFile $Destination -UseBasicParsing
        }
    }

    $target = Get-Target
    if ($PrintTarget) {
        Write-Output $target
        return
    }

    # TLS 1.2 for GitHub on Windows PowerShell 5.1, whose default may be older.
    if ($PSVersionTable.PSVersion.Major -lt 6) {
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    }

    $base = if ($env:OPENBIM_IFC_BASE_URL) { $env:OPENBIM_IFC_BASE_URL.TrimEnd('/') } else { "https://github.com/$Repo/releases/download" }
    if (-not $Prefix) {
        if (-not $env:LOCALAPPDATA) { Fail 'LOCALAPPDATA is not set; pass -Prefix' }
        $Prefix = Join-Path $env:LOCALAPPDATA 'Programs\openbim-ifc'
    }
    if ($env:OPENBIM_IFC_NO_MODIFY_PATH -eq '1') { $NoModifyPath = $true }

    # The newest openbim-ifc-cli release: other crates of the repository
    # release under their own tags, so "latest" is not necessarily this one.
    if (-not $Version) {
        try {
            $releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases?per_page=100" -UseBasicParsing
        } catch {
            Fail "could not list the releases of ${Repo}: $($_.Exception.Message); pass -Version"
        }
        $newest = @($releases | Where-Object { $_.tag_name -like "$Crate-v*" }) | Select-Object -First 1
        if (-not $newest) { Fail "no $Crate release found; pass -Version" }
        $Version = $newest.tag_name
    }
    if ($Version.StartsWith("$Crate-")) { $Version = $Version.Substring($Crate.Length + 1) }
    $Version = $Version.TrimStart('v')

    $tag = "$Crate-v$Version"
    $stem = "$Bin-v$Version-$target"
    $archive = "$stem.zip"
    $work = Join-Path ([IO.Path]::GetTempPath()) ("openbim-ifc-install-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $work | Out-Null
    try {
        Say "downloading $archive ($tag)"
        try {
            Get-Download "$base/$tag/$archive" (Join-Path $work $archive)
        } catch {
            Fail "could not download $base/$tag/${archive}: $($_.Exception.Message)"
        }
        try {
            Get-Download "$base/$tag/SHA256SUMS" (Join-Path $work 'SHA256SUMS')
        } catch {
            Fail "could not download $base/$tag/SHA256SUMS: $($_.Exception.Message)"
        }

        $expected = $null
        foreach ($line in Get-Content -LiteralPath (Join-Path $work 'SHA256SUMS')) {
            $fields = $line.Trim() -split '\s+', 2
            if ($fields.Count -eq 2 -and $fields[1].TrimStart('*') -eq $archive) {
                $expected = $fields[0].ToLowerInvariant()
                break
            }
        }
        if (-not $expected) { Fail "SHA256SUMS lists no $archive" }
        $actual = (Get-FileHash -LiteralPath (Join-Path $work $archive) -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($expected -ne $actual) {
            Fail "checksum mismatch for ${archive}: expected $expected, got $actual; nothing installed"
        }

        Expand-Archive -LiteralPath (Join-Path $work $archive) -DestinationPath (Join-Path $work 'unpacked')
        $exe = Join-Path $work "unpacked\$stem\$Bin.exe"
        if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { Fail "$archive holds no $stem\$Bin.exe" }

        $binDir = Join-Path $Prefix 'bin'
        New-Item -ItemType Directory -Force -Path $binDir | Out-Null
        $installed = Join-Path $binDir "$Bin.exe"
        Copy-Item -LiteralPath $exe -Destination "$installed.tmp" -Force
        Move-Item -LiteralPath "$installed.tmp" -Destination $installed -Force
        $reported = & $installed --version
        if ($LASTEXITCODE -ne 0) { Fail "the installed $installed did not run" }
        Say "installed $reported to $installed"
    } finally {
        Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    }

    # PATH: the user's, persistent, and this session's.
    $binDir = (Resolve-Path -LiteralPath (Join-Path $Prefix 'bin')).Path
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @(if ($userPath) { $userPath -split ';' | Where-Object { $_ } })
    $onPath = @($entries | Where-Object { $_.TrimEnd('\') -ieq $binDir.TrimEnd('\') }).Count -gt 0
    if ($onPath) { return }
    if ($NoModifyPath) {
        Say "$binDir is not on the user PATH; add it to run $Bin from any shell"
        return
    }
    $add = [bool] $AddToPath
    if (-not $add) {
        $interactive = [Environment]::UserInteractive -and -not [Console]::IsInputRedirected
        if ($interactive) {
            $choices = [System.Management.Automation.Host.ChoiceDescription[]] @('&Yes', '&No')
            $add = $Host.UI.PromptForChoice('openbim-ifc', "Add $binDir to your user PATH?", $choices, 0) -eq 0
        } else {
            Say "$binDir is not on the user PATH; run again with -AddToPath, or add it yourself"
            return
        }
    }
    if ($add) {
        $newPath = (@($entries) + $binDir) -join ';'
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        $env:Path = "$env:Path;$binDir"
        Say "added $binDir to the user PATH; new shells will find $Bin"
    }
}
