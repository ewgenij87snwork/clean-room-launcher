param(
  [Parameter(Mandatory = $true)][string]$Artifact,
  [Parameter(Mandatory = $true)][string]$Sha256,
  [Parameter(Mandatory = $true)][string]$Receipt
)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $Artifact -PathType Leaf)) { throw 'P08_LIVE_OS_REFUSED:ARTIFACT_MISSING' }
$actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $Artifact).Hash.ToLowerInvariant()
if ($actual -ne $Sha256) { throw 'P08_LIVE_OS_REFUSED:ARTIFACT_CHECKSUM_MISMATCH' }
function Digest-Or-Unavailable([string]$value, [string]$label) {
  if ([string]::IsNullOrEmpty($value)) { return 'UNAVAILABLE' }
  if ($value -notmatch '^[0-9a-f]{64}$') { throw "P08_LIVE_OS_REFUSED:INVALID_$label" }
  return $value
}
$config = Digest-Or-Unavailable $env:TASKSEAL_CONFIG_SHA256 'CONFIG_SHA256'
$provider = Digest-Or-Unavailable $env:TASKSEAL_PROVIDER_SHA256 'PROVIDER_SHA256'
$gitState = Digest-Or-Unavailable $env:TASKSEAL_GIT_SHA256 'GIT_SHA256'
$userFiles = Digest-Or-Unavailable $env:TASKSEAL_USER_FILES_SHA256 'USER_FILES_SHA256'
foreach ($entry in @(@($config, $env:TASKSEAL_CONFIG_SHA256_AFTER), @($provider, $env:TASKSEAL_PROVIDER_SHA256_AFTER), @($gitState, $env:TASKSEAL_GIT_SHA256_AFTER), @($userFiles, $env:TASKSEAL_USER_FILES_SHA256_AFTER))) {
  if (-not [string]::IsNullOrEmpty($entry[1]) -and $entry[0] -ne $entry[1]) { throw 'P08_LIVE_OS_REFUSED:PROTECTED_STATE_MISMATCH' }
}
$prerequisites = Digest-Or-Unavailable $env:TASKSEAL_PREREQUISITES_SHA256 'PREREQUISITES_SHA256'
$record = [ordered]@{
  schema_version = 'taskseal.live-os-receipt.v1'; lane = 'windows'; qualification = 'NOT_QUALIFIED'; artifact_sha256 = $actual
  clean_image = [ordered]@{ id = $(if ($env:TASKSEAL_CLEAN_IMAGE_ID) { $env:TASKSEAL_CLEAN_IMAGE_ID } else { 'UNAVAILABLE' }); verified = $false }
  prerequisites = [ordered]@{ sha256 = $prerequisites; verified = $false }
  lifecycle = [ordered]@{ install = 'NOT_RUN'; run = 'NOT_RUN'; upgrade = 'NOT_RUN'; rollback = 'NOT_RUN'; uninstall = 'NOT_RUN' }
  protected_state_before = [ordered]@{ config_sha256 = $config; provider_sha256 = $provider; git_sha256 = $gitState; user_files_sha256 = $userFiles }
  protected_state_after = [ordered]@{ config_sha256 = $config; provider_sha256 = $provider; git_sha256 = $gitState; user_files_sha256 = $userFiles }
  reason = 'clean_image_or_prerequisites_not_verified'
}
$payload = $record | ConvertTo-Json -Compress -Depth 4
$record.receipt_sha256 = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($payload))).ToLowerInvariant()
$record | ConvertTo-Json -Compress -Depth 4 | Set-Content -LiteralPath $Receipt -NoNewline -Encoding utf8
Write-Output 'P08_LIVE_OS_NOT_QUALIFIED lane=windows'
