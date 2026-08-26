#!/usr/bin/env bash
set -euo pipefail

if [ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]; then
  PROJECT_DIR="$BUILD_WORKSPACE_DIRECTORY/tools/benches/dotnet"
  WORKSPACE_ROOT="$BUILD_WORKSPACE_DIRECTORY"
else
  PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  WORKSPACE_ROOT="$(cd "$PROJECT_DIR/../../.." && pwd)"
fi

VERSION="${1:?usage: run.sh <ArenaDotnet.Xunit version> [iterations]}"
ITERATIONS="${2:-10}"

bazel_output_base() {
  readlink -f "$WORKSPACE_ROOT/bazel-out" 2>/dev/null | sed 's#/execroot/.*##'
}

if command -v dotnet >/dev/null 2>&1; then
  DOTNET="dotnet"
else
  OUTPUT_BASE="$(bazel_output_base)"
  DOTNET_DIR=""
  for candidate in "$OUTPUT_BASE"/external/*dotnet*x86_64-unknown-linux-gnu* "$OUTPUT_BASE"/external/*dotnet*linux*; do
    if [ -x "$candidate/dotnet" ]; then
      DOTNET_DIR="$candidate"
      break
    fi
  done
  if [ -z "$DOTNET_DIR" ]; then
    echo "run.sh: no system dotnet on PATH and could not locate a Bazel-fetched dotnet SDK repo" >&2
    echo "run.sh: run 'bazel build @rules_dotnet//dotnet:sdk' or build any dotnet target once, then retry" >&2
    exit 1
  fi
  export DOTNET_ROOT="$DOTNET_DIR"
  export PATH="$DOTNET_ROOT:$PATH"
  DOTNET="$DOTNET_ROOT/dotnet"
fi

export DOTNET_NOLOGO=1
export DOTNET_CLI_TELEMETRY_OPTOUT=1

cd "$PROJECT_DIR"
"$DOTNET" run -c Release -p:BenchArtifactVersion="$VERSION" -- "$VERSION" "$ITERATIONS"
