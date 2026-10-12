[CmdletBinding()]
# KeepBuildCache remains accepted for existing callers; retention is now default.
param([switch]$TestOnly, [switch]$KeepBuildCache)
$ErrorActionPreference = 'Stop'
$project = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$desktop = Join-Path $project 'apps/desktop'
$build = Join-Path $project 'build'
$oldTarget = $env:CARGO_TARGET_DIR
Push-Location $desktop
try {
    & pnpm install --frozen-lockfile
    if ($LASTEXITCODE -ne 0) { throw 'Desktop dependency installation failed' }
    & pnpm check
    if ($LASTEXITCODE -ne 0) { throw 'Desktop type check failed' }
    $env:CARGO_TARGET_DIR = $build
    & node (Join-Path $desktop 'node_modules/@tauri-apps/cli/tauri.js') build --no-bundle -- --locked
    if ($LASTEXITCODE -ne 0) { throw 'Desktop build failed; installed app unchanged' }
    $candidate = Join-Path $build 'release/TajsAnagrams.exe'
    if (-not (Test-Path -LiteralPath $candidate)) { throw 'Desktop executable missing' }
    if ($TestOnly -or $env:CI) { Write-Output "Test/CI executable (not installed or restarted): $candidate"; return }

    . (Join-Path $PSScriptRoot 'desktop-installation.ps1')
    $plan = New-DesktopInstallPlan -Project $project -LocalData ([Environment]::GetFolderPath('LocalApplicationData')) -Programs ([Environment]::GetFolderPath('Programs'))
    Invoke-DesktopInstallation -Plan $plan -Candidate $candidate -StopApps {
        param($plan)
        foreach ($name in @('TajsAnagrams', 'AnagramSolver')) {
            $expected = if ($name -eq 'TajsAnagrams') { Join-Path $plan.Install "$name.exe" } else { Join-Path $plan.Legacy "$name.exe" }
            Get-Process -Name $name -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $expected } | ForEach-Object {
                if (-not $_.CloseMainWindow() -or -not $_.WaitForExit(15000)) { throw "Close $name before installing; no process was forcibly terminated." }
            }
        }
    } -CreateShortcut {
        param($exe, $install, $link)
        $shell = New-Object -ComObject WScript.Shell
        $shortcut = $shell.CreateShortcut($link)
        $shortcut.TargetPath = $exe
        $shortcut.WorkingDirectory = $install
        $shortcut.Description = 'Solve anagrams locally'
        $shortcut.Save()
    } -Activate {
        param($exe, $install)
        # Requested interactive application, not a background helper.
        $process = Start-Process -FilePath $exe -WorkingDirectory $install -WindowStyle Normal -PassThru
        Start-Sleep -Seconds 3
        if ($process.HasExited) { throw 'New app exited during startup.' }
        Write-Output "Installed and started: $exe (PID $($process.Id), SHA256 $((Get-FileHash -LiteralPath $exe).Hash))"
    }
    Write-Output "Cargo build cache retained: $build; installed release rotation is unchanged."
} finally {
    $env:CARGO_TARGET_DIR = $oldTarget
    Pop-Location
}
