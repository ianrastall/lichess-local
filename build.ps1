param([switch]$SkipTests)
$ErrorActionPreference='Stop'
Push-Location $PSScriptRoot
try {
 if (-not (Test-Path upstream-manifest.json) -or -not (Test-Path artifacts/engine/fairy-stockfish.exe)) {
  python tools/prepare-assets.py
  if ($LASTEXITCODE) { throw 'Upstream preparation failed.' }
 }
 python tools/verify-assets.py
 if ($LASTEXITCODE) { throw 'Asset validation failed.' }
 cargo build --release --locked --manifest-path core/Cargo.toml --target-dir artifacts/rust
 if ($LASTEXITCODE) { throw 'Rust build failed.' }
 if (-not $SkipTests) {
  cargo test --release --locked --manifest-path core/Cargo.toml --target-dir artifacts/rust
  if ($LASTEXITCODE) { throw 'Rust tests failed.' }
  python tools/smoke-core.py
  if ($LASTEXITCODE) { throw 'Live engine checks failed.' }
  dotnet run --project tests/InteropSmoke -c Release -- $PSScriptRoot
  if ($LASTEXITCODE) { throw 'Managed/native binding checks failed.' }
 }
 Push-Location web
 try {
  npm ci
  if ($LASTEXITCODE) { throw 'npm restore failed.' }
  npm audit --audit-level=high
  if ($LASTEXITCODE) { throw 'Dependency audit failed.' }
  python ../tools/stage-notices.py
  if ($LASTEXITCODE) { throw 'Third-party notices failed.' }
  npm run build
  if ($LASTEXITCODE) { throw 'Web view build failed.' }
  if (-not $SkipTests) { npm run test:browser; if ($LASTEXITCODE) { throw 'Browser checks failed.' } }
 } finally { Pop-Location }
 dotnet publish app/LichessLocal.csproj -c Release -r win-x64 --self-contained true -o artifacts/package --nologo -v:q
 if ($LASTEXITCODE) { throw 'Desktop build/style checks failed.' }
 foreach ($file in 'LichessLocal.exe','LichessLocal.pri','MainWindow.xbf','merci_core.dll','Web/index.html','Engine/fairy-stockfish.exe') {
  if (-not (Test-Path (Join-Path artifacts/package $file))) { throw "Publish output missing $file" }
 }
 $runtime='artifacts/downloads/MicrosoftEdgeWebView2RuntimeInstallerX64.exe'
 if (-not (Test-Path $runtime)) { curl.exe --fail --location --silent --show-error --output $runtime 'https://go.microsoft.com/fwlink/?linkid=2124701'; if ($LASTEXITCODE) { throw 'Offline runtime download failed.' } }
 $signature=Get-AuthenticodeSignature $runtime
 if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'CN=Microsoft Corporation') { throw 'Offline runtime signature is not valid.' }
 $iscc=Join-Path $env:LOCALAPPDATA 'Programs/Inno Setup 6/ISCC.exe'
 if (-not (Test-Path $iscc)) { $iscc="${env:ProgramFiles(x86)}/Inno Setup 6/ISCC.exe" }
 & $iscc /Qp "/DSourceDir=$PSScriptRoot/artifacts/package" "/DRuntimeInstaller=$PSScriptRoot/$runtime" installer/LichessLocal.iss
 if ($LASTEXITCODE) { throw 'Installer creation failed.' }
 python tools/package-source.py
 if ($LASTEXITCODE) { throw 'Source package creation failed.' }
 Get-FileHash artifacts/installer/LichessLocal-0.1.0-Setup.exe,artifacts/LichessLocal-0.1.0-source.zip | Format-Table Path,Hash
} finally { Pop-Location }
