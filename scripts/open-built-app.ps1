param(
    [switch]$PrintPath
)

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$candidates = @(
    (Join-Path $repositoryRoot "src-tauri\target\release\mr-manager.exe"),
    (Join-Path $repositoryRoot "src-tauri\target\debug\mr-manager.exe")
)
$executable = $candidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1

if (-not $executable) {
    Write-Error "No built Mr Manager executable was found. Run 'npm run app:build' first."
    exit 1
}

$resolvedExecutable = (Resolve-Path -LiteralPath $executable).Path

if ($PrintPath) {
    Write-Output $resolvedExecutable
    exit 0
}

Start-Process -FilePath $resolvedExecutable -WorkingDirectory $repositoryRoot | Out-Null
