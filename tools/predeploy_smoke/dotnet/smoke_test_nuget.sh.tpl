#!/usr/bin/env bash
# --- begin runfiles.bash initialization v3 ---
set -uo pipefail; set +e; f=bazel_tools/tools/bash/runfiles/runfiles.bash
source "${RUNFILES_DIR:-/dev/null}/$f" 2>/dev/null || \
  source "$(grep -sm1 "^$f " "${RUNFILES_MANIFEST_FILE:-/dev/null}" | cut -f2- -d' ')" 2>/dev/null || \
  source "$0.runfiles/$f" 2>/dev/null || \
  source "$(grep -sm1 "^$f " "$0.runfiles_manifest" | cut -f2- -d' ')" 2>/dev/null || \
  source "$(grep -sm1 "^$f " "$0.exe.runfiles_manifest" | cut -f2- -d' ')" 2>/dev/null || \
  { echo>&2 "ERROR: cannot find $f"; exit 1; }; f=; set -e
runfiles_export_envvars
# --- end runfiles.bash initialization v3 ---

set -o pipefail -o errexit -o nounset

DOTNET="$(rlocation TEMPLATED_dotnet)"
PACKAGE="$(rlocation TEMPLATED_nupkg)"
PACKAGE_ID="TEMPLATED_nuget_id"

export DOTNET_MULTILEVEL_LOOKUP="false"
export DOTNET_NOLOGO="1"
export DOTNET_CLI_TELEMETRY_OPTOUT="1"
export DOTNET_ROOT
DOTNET_ROOT="$(dirname "$DOTNET")"

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

VERSION="$(unzip -p "$PACKAGE" "*.nuspec" | grep -oE '<version>[^<]+' | sed -E 's/<version>//')"
if [ -z "$VERSION" ]; then
  echo "smoke test: could not read <version> from $PACKAGE's nuspec" >&2
  exit 1
fi

FEED_DIR="$WORKDIR/localfeed"
mkdir -p "$FEED_DIR"
LOWER_ID="$(echo "$PACKAGE_ID" | tr '[:upper:]' '[:lower:]')"
cp "$PACKAGE" "$FEED_DIR/${LOWER_ID}.${VERSION}.nupkg"

PROJECT_DIR="$WORKDIR/probe"
mkdir -p "$PROJECT_DIR"

cat > "$PROJECT_DIR/NuGet.config" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<configuration>
  <packageSources>
    <clear />
    <add key="local-arena" value="$FEED_DIR" />
    <add key="nuget.org" value="https://api.nuget.org/v3/index.json" />
  </packageSources>
</configuration>
EOF

cat > "$PROJECT_DIR/probe.csproj" <<EOF
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
    <IsPackable>false</IsPackable>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="$PACKAGE_ID" Version="$VERSION" />
    <PackageReference Include="Microsoft.NET.Test.Sdk" Version="17.11.0" />
    <PackageReference Include="xunit" Version="2.9.3" />
    <PackageReference Include="xunit.runner.visualstudio" Version="3.1.5" />
  </ItemGroup>
</Project>
EOF

cat > "$PROJECT_DIR/SmokeTest.cs" <<'CSEOF'
using System;
using System.Net;
using System.Net.Http;
using System.Net.Sockets;
using System.Threading.Tasks;
using ArenaDotnet.Xunit;
using Xunit;

public class SmokeTest
{
    [Fact]
    public void OpenAndCloseArena_RealNugetPackage_Succeeds()
    {
        using var fixture = new SmokeTestFixture();
        Assert.NotNull(fixture.Arena);
    }

    [Fact]
    public async Task Dashboard_RealNugetPackage_ServesDashboard()
    {
        var port = PickFreePort();
        var closed = new ClosedArena(
                "nuget-dashboard-smoke-test-arena",
                new MatchBuilder("nuget-dashboard-smoke-test-match").Build())
            .WithLifecycleDashboard(port, null);

        var arena = await closed.OpenAsync();
        try
        {
            var index = await ReadAsync(port, "/");
            Assert.NotNull(index);
            Assert.Contains("bundle.js", index);
            Assert.NotNull(await ReadAsync(port, "/bundle.js"));
            Assert.NotNull(await ReadAsync(port, "/styles.css"));
        }
        finally
        {
            arena.Dispose();
            await StopDashboardAsync(port);
        }
    }

    private static int PickFreePort()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        var port = ((IPEndPoint)listener.LocalEndpoint).Port;
        listener.Stop();
        return port;
    }

    private static async Task<string?> ReadAsync(int port, string path)
    {
        using var client = new HttpClient { Timeout = TimeSpan.FromSeconds(10) };
        try
        {
            var response = await client.GetAsync($"http://127.0.0.1:{port}{path}");
            if (!response.IsSuccessStatusCode)
                return null;
            var body = await response.Content.ReadAsStringAsync();
            return body.Length == 0 ? null : body;
        }
        catch (HttpRequestException)
        {
            return null;
        }
    }

    private static async Task StopDashboardAsync(int port)
    {
        using var client = new HttpClient { Timeout = TimeSpan.FromSeconds(10) };
        try
        {
            await client.PostAsync($"http://127.0.0.1:{port}/shutdown", null);
        }
        catch (HttpRequestException)
        {
        }
    }
}

sealed class SmokeTestFixture : ArenaCollectionFixture
{
    protected override Match Configure() => new MatchBuilder("nuget-smoke-test-match").Build();
}
CSEOF

cd "$PROJECT_DIR"
set +e
OUTPUT="$("$DOTNET" test 2>&1)"
STATUS=$?
set -e
echo "$OUTPUT"

if [ "$STATUS" -ne 0 ]; then
  echo "smoke test FAILED: dotnet test exited $STATUS" >&2
  exit 1
fi
if ! grep -q "Passed!" <<<"$OUTPUT"; then
  echo "smoke test FAILED: dotnet test exited 0 but did not report a passing test run (possible zero-test discovery)" >&2
  exit 1
fi

echo "smoke test PASSED: $PACKAGE_ID $VERSION restored from NuGet via a real xunit test project run with 'dotnet test', opened and closed a real arena, and served the packaged dashboard"
