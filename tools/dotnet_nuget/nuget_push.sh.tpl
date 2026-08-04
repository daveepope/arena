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

export DOTNET_MULTILEVEL_LOOKUP="false"
export DOTNET_NOLOGO="1"
export DOTNET_CLI_TELEMETRY_OPTOUT="1"
export DOTNET_ROOT="$(dirname "$(rlocation TEMPLATED_dotnet)")"

: "${NUGET_API_KEY:?NUGET_API_KEY must be set (e.g. from the NuGet/login OIDC step)}"
: "${NUGET_SOURCE:?NUGET_SOURCE must be set, e.g. https://api.nuget.org/v3/index.json}"

exec "$(rlocation TEMPLATED_dotnet)" nuget push "$(rlocation TEMPLATED_package)" \
  --source "$NUGET_SOURCE" --api-key "$NUGET_API_KEY" --skip-duplicate
