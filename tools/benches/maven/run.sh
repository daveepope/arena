#!/usr/bin/env bash
set -euo pipefail

if [ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]; then
  PROJECT_DIR="$BUILD_WORKSPACE_DIRECTORY/tools/benches/maven"
  WORKSPACE_ROOT="$BUILD_WORKSPACE_DIRECTORY"
else
  PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  WORKSPACE_ROOT="$(cd "$PROJECT_DIR/../../.." && pwd)"
fi

VERSION="${1:?usage: run.sh <arena-junit version> [iterations]}"
ITERATIONS="${2:-10}"

bazel_output_base() {
  readlink -f "$WORKSPACE_ROOT/bazel-out" 2>/dev/null | sed 's#/execroot/.*##'
}

if command -v mvn >/dev/null 2>&1; then
  MVN="mvn"
else
  OUTPUT_BASE="$(bazel_output_base)"
  MVN_DIR="$(find "$OUTPUT_BASE/external" -maxdepth 1 -iname "*apache_maven*" -type d 2>/dev/null | head -1)"
  if [ -z "$MVN_DIR" ]; then
    echo "run.sh: no system mvn on PATH and could not locate the Bazel-fetched apache_maven repo" >&2
    echo "run.sh: run 'bazel build @apache_maven//:dist' once to fetch it, then retry" >&2
    exit 1
  fi
  MVN="$MVN_DIR/bin/mvn"
fi

if ! command -v javac >/dev/null 2>&1; then
  OUTPUT_BASE="$(bazel_output_base)"
  JDK_DIR=""
  for candidate in "$OUTPUT_BASE"/external/*remotejdk25_linux*; do
    if [ -x "$candidate/bin/javac" ]; then
      JDK_DIR="$candidate"
      break
    fi
  done
  if [ -z "$JDK_DIR" ]; then
    echo "run.sh: no system javac and could not locate a Bazel-fetched JDK 25 repo" >&2
    echo "run.sh: run 'bazel build @rules_java//toolchains:all' once to fetch one, then retry" >&2
    exit 1
  fi
  export JAVA_HOME="$JDK_DIR"
  export PATH="$JAVA_HOME/bin:$PATH"
fi

cd "$PROJECT_DIR"
"$MVN" --batch-mode -q \
  -Dbench.artifact.version="$VERSION" \
  compile org.codehaus.mojo:exec-maven-plugin:3.5.0:java \
  -Dexec.mainClass=bench.Bench \
  -Dexec.args="$VERSION $ITERATIONS"
