#Requires -Version 5.1
param(
  [ValidateSet("install", "installMulti", "start", "stop", "list")]
  [string]$Action = "install",
  [string]$BundleName = "",
  [string]$AbilityName = "EntryAbility",
  [string]$OutputDir = "",
  [string]$Udid = "",
  # Format: name|relPath,name|relPath  (for installMulti)
  [string]$ModuleSpecs = ""
)

$ErrorActionPreference = "Stop"

function Ensure-Hdc {
  $hdc = Get-Command hdc -ErrorAction SilentlyContinue
  if (-not $hdc) {
    throw "hdc not found. Open a new Cursor terminal so PATH includes DevEco toolchains."
  }
}

function Get-DeviceUdid {
  param([string]$Preferred)
  if ($Preferred) { return $Preferred }

  $raw = (& hdc list targets 2>&1 | Out-String)
  $lines = @()
  foreach ($line in ($raw -split [Environment]::NewLine)) {
    $t = $line.Trim()
    if (-not $t) { continue }
    if ($t -eq "[Empty]") { continue }
    if ($t.StartsWith("[")) { continue }
    if ($t -match "List of targets") { continue }
    $lines += $t
  }

  if ($lines.Count -eq 0) {
    throw "No device found. Connect USB, enable debugging, then run: hdc list targets"
  }
  if ($lines.Count -gt 1) {
    Write-Host "Multiple devices found, using the first one. Pass -Udid to select:"
    foreach ($d in $lines) { Write-Host ("  " + $d) }
  }
  return $lines[0]
}

function Find-SignedOutputDir {
  param([string]$Root, [string]$Hint)
  if ($Hint -and (Test-Path $Hint)) {
    return (Resolve-Path $Hint).Path
  }

  # Prefer entry module HAP; do not pick outputs\components\* side products
  $entryCandidates = @(
    (Join-Path $Root "entry\build\default\outputs\default"),
    (Join-Path $Root "products\phone\entry\build\default\outputs\default")
  )
  foreach ($c in $entryCandidates) {
    if (-not (Test-Path $c)) { continue }
    $has = Get-ChildItem $c -Filter "*-signed.hap" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($has) {
      return (Resolve-Path $c).Path
    }
  }

  $candidates = @(
    (Join-Path $Root "outputs\default\signed"),
    (Join-Path $Root "outputs\project\bundles\signed"),
    (Join-Path $Root "outputs\default")
  )
  foreach ($c in $candidates) {
    if (-not (Test-Path $c)) { continue }
    $has = Get-ChildItem $c -Filter "*-signed.hap" -Recurse -ErrorAction SilentlyContinue |
      Where-Object { $_.FullName -notmatch '[\\/]components[\\/]' } |
      Select-Object -First 1
    if ($has) {
      return $has.Directory.FullName
    }
  }

  # Last resort: newest signed hap under outputs, still skip components/
  $outputsRoot = Join-Path $Root "outputs"
  if (Test-Path $outputsRoot) {
    $hap = Get-ChildItem $outputsRoot -Filter "*-signed.hap" -Recurse -ErrorAction SilentlyContinue |
      Where-Object { $_.FullName -notmatch '[\\/]components[\\/]' } |
      Sort-Object LastWriteTime -Descending |
      Select-Object -First 1
    if ($hap) {
      return $hap.Directory.FullName
    }
  }

  throw "No *-signed.hap found (skipped outputs/components). Build Multi or assembleApp first, or pass -OutputDir."
}

function Install-FromDir {
  param([string]$Id, [string]$SignedDir, [string]$Bundle)
  $remote = "/data/local/tmp/" + $Bundle
  Write-Host ("UDID   : " + $Id)
  Write-Host ("Bundle : " + $Bundle)
  Write-Host ("Local  : " + $SignedDir)
  Write-Host ("Remote : " + $remote)

  hdc -t $Id shell ("rm -rf " + $remote + " && mkdir -p " + $remote)
  hdc -t $Id file send $SignedDir ($remote + "/")
  $leaf = Split-Path $SignedDir -Leaf
  $installPath = $remote + "/" + $leaf
  Write-Host ("bm install -p " + $installPath)
  hdc -t $Id shell ("bm install -p " + $installPath)
  Write-Host "Install done."
}

function Collect-MultiSigned {
  param([string]$Root, [string]$Specs)
  if (-not $Specs) { throw "ModuleSpecs is empty" }

  $staging = Join-Path $env:TEMP ("harmony-multi-deploy-" + [guid]::NewGuid().ToString("N"))
  New-Item -ItemType Directory -Force -Path $staging | Out-Null
  $count = 0

  foreach ($part in ($Specs -split ",")) {
    $part = $part.Trim()
    if (-not $part) { continue }
    $bits = $part -split "\|", 2
    if ($bits.Count -lt 2) { continue }
    $name = $bits[0]
    $rel = $bits[1] -replace "^\./", "" -replace "/", "\"
    $outDir = Join-Path $Root (Join-Path $rel "build\default\outputs\default")
    if (-not (Test-Path $outDir)) {
      Write-Host ("SKIP " + $name + " (no output): " + $outDir)
      continue
    }
    $files = @()
    $files += Get-ChildItem $outDir -Filter "*-signed.hap" -ErrorAction SilentlyContinue
    $files += Get-ChildItem $outDir -Filter "*-signed.hsp" -ErrorAction SilentlyContinue
    if ($files.Count -eq 0) {
      Write-Host ("SKIP " + $name + " (no signed hap/hsp)")
      continue
    }
    foreach ($f in $files) {
      Copy-Item $f.FullName (Join-Path $staging $f.Name) -Force
      Write-Host ("ADD  " + $f.Name)
      $count++
    }
  }

  if ($count -eq 0) {
    Remove-Item -Recurse -Force $staging -ErrorAction SilentlyContinue
    throw "No signed HAP/HSP collected. Build Multi Deploy modules first."
  }
  return $staging
}

Ensure-Hdc
if ($Action -ne "list" -and -not $BundleName) {
  throw "BundleName is empty. Configure harmonyos.bundleName or ensure AppScope/app.json5 defines app.bundleName."
}
# Prefer workspace from .vscode parent; else current directory (extension sets cwd to project root)
if ((Split-Path $PSScriptRoot -Leaf) -eq ".vscode") {
  $workspace = Split-Path $PSScriptRoot -Parent
} else {
  $workspace = (Get-Location).Path
}

switch ($Action) {
  "list" {
    hdc list targets
    break
  }
  "stop" {
    $id = Get-DeviceUdid -Preferred $Udid
    Write-Host ("force-stop " + $BundleName + " on " + $id)
    hdc -t $id shell ("aa force-stop " + $BundleName)
    break
  }
  "start" {
    $id = Get-DeviceUdid -Preferred $Udid
    Write-Host ("start " + $AbilityName + " / " + $BundleName + " on " + $id)
    hdc -t $id shell ("aa start -a " + $AbilityName + " -b " + $BundleName)
    break
  }
  "install" {
    $id = Get-DeviceUdid -Preferred $Udid
    $signed = Find-SignedOutputDir -Root $workspace -Hint $OutputDir
    Install-FromDir -Id $id -SignedDir $signed -Bundle $BundleName
    break
  }
  "installMulti" {
    $id = Get-DeviceUdid -Preferred $Udid
    $staging = Collect-MultiSigned -Root $workspace -Specs $ModuleSpecs
    try {
      Install-FromDir -Id $id -SignedDir $staging -Bundle $BundleName
    } finally {
      Remove-Item -Recurse -Force $staging -ErrorAction SilentlyContinue
    }
    break
  }
}
