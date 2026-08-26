$ErrorActionPreference = 'Stop'

$wslConfigPath = Join-Path $Env:USERPROFILE ".wslconfig"
if (-not (Test-Path $wslConfigPath)) {
  Set-Content -Path $wslConfigPath -Value "[wsl2]`nmemory=5GB`nvmIdleTimeout=-1" -Encoding ASCII
}

wsl.exe --set-default-version 2 | Out-Null

$installed = ((wsl.exe --list --quiet) -replace "`0", "") | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne "" }
if ($installed -notcontains "Ubuntu") {
  wsl.exe --install Ubuntu --no-launch
  if ($LASTEXITCODE -ne 0) { throw "wsl --install Ubuntu failed" }
}

function Wsl-Run([string]$Command) {
  wsl.exe --distribution Ubuntu --user root -- bash -c $Command
  if ($LASTEXITCODE -ne 0) { throw "WSL command failed ($LASTEXITCODE): $Command" }
}

# WSL2's kernel has bridge and br_netfilter compiled directly in, but ships no
# /lib/modules metadata at all, so Docker's modprobe sanity-check before enabling the
# bridge network driver fails and Docker silently drops "bridge" from its plugin
# registry ("could not find plugin bridge in v1 plugin registry"). Declaring them
# built-in lets modprobe report success without needing an actual loadable module.
Wsl-Run "mkdir -p /lib/modules/`$(uname -r) && printf 'kernel/net/bridge/bridge.ko\nkernel/net/bridge/br_netfilter.ko\n' > /lib/modules/`$(uname -r)/modules.builtin && depmod `$(uname -r)"

Wsl-Run "command -v docker >/dev/null 2>&1 || (apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --yes docker.io)"

# Docker's default bridge (172.17.0.0/16) can collide with WSL2's randomly assigned NAT
# subnet (172.16.0.0/12), breaking host<->container routing, so pin both to 10.x. Bind the
# API to loopback only: WSL2's localhost-forwarding auto-exposes 127.0.0.1 listeners inside
# the distro at 127.0.0.1 on the Windows host, so the native Windows-side Rust test binaries
# (bollard) can reach it via DOCKER_HOST without exposing an unauthenticated API on the
# WSL2 virtual network.
$daemonConfig = @{
  bip                     = "10.11.0.1/24"
  "default-address-pools" = @(
    @{ base = "10.12.0.0/16"; size = 24 }
    @{ base = "10.13.0.0/16"; size = 24 }
  )
  hosts                   = @("unix:///var/run/docker.sock", "tcp://127.0.0.1:2375")
}
$daemonJson = $daemonConfig | ConvertTo-Json -Depth 5 -Compress
$daemonJsonB64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($daemonJson))
Wsl-Run "mkdir -p /etc/docker && echo $daemonJsonB64 | base64 -d > /etc/docker/daemon.json"

# dockerd refuses to start if -H is set both via daemon.json ("hosts") and the systemd
# unit's own ExecStart (docker.io ships with `-H fd://` baked in), so drop a systemd
# override clearing the unit's own -H flag.
Wsl-Run "mkdir -p /etc/systemd/system/docker.service.d && printf '[Service]\nExecStart=\nExecStart=/usr/bin/dockerd\n' > /etc/systemd/system/docker.service.d/override.conf"

Wsl-Run "if [ -d /run/systemd/system ]; then systemctl daemon-reload && systemctl restart docker; else service docker restart; fi"

# WSL terminates a distribution when no process remains under its init; a D-Bus session bus
# launched via `wsl --exec` keeps it (and dockerd) alive for the rest of the job.
Wsl-Run "command -v dbus-launch >/dev/null 2>&1 || (apt-get update && apt-get install -y dbus-x11)"
wsl.exe --distribution Ubuntu --user root --exec /usr/bin/dbus-launch true
if ($LASTEXITCODE -ne 0) { throw "dbus-launch keep-alive failed" }

Write-Output "Verifying Docker is reachable at tcp://127.0.0.1:2375"
Invoke-WebRequest -Uri "http://127.0.0.1:2375/version" -UseBasicParsing | Out-Null

Add-Content -Path $Env:GITHUB_ENV -Value "DOCKER_HOST=tcp://127.0.0.1:2375"
