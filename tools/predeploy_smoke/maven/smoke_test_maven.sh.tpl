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

MVN="$(rlocation TEMPLATED_mvn)"
JAVA_BIN="$(rlocation TEMPLATED_java_executable)"
JAR="$(rlocation TEMPLATED_jar)"
POM="$(rlocation TEMPLATED_pom)"
NATIVE_JAR="$(rlocation TEMPLATED_native_jar)"
NATIVE_CLASSIFIER="TEMPLATED_native_classifier"
GROUP_ID="TEMPLATED_group_id"
ARTIFACT_ID="TEMPLATED_artifact_id"

export JAVA_HOME
JAVA_HOME="$(cd "$(dirname "$JAVA_BIN")/.." && pwd)"
export PATH="$JAVA_HOME/bin:$PATH"

VERSION="$(grep -oE '<version>[^<]+</version>' "$POM" | head -1 | sed -E 's#</?version>##g')"
if [ -z "$VERSION" ]; then
  echo "smoke test: could not read <version> from $POM" >&2
  exit 1
fi

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

LOCAL_REPO="$WORKDIR/local-repo"
mkdir -p "$LOCAL_REPO"

INSTALLER_DIR="$WORKDIR/installer"
mkdir -p "$INSTALLER_DIR"
cat > "$INSTALLER_DIR/pom.xml" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
  <modelVersion>4.0.0</modelVersion>
  <groupId>arena.smoketest</groupId>
  <artifactId>installer</artifactId>
  <version>0.0.0</version>
  <packaging>pom</packaging>
  <build>
    <plugins>
      <plugin>
        <groupId>org.apache.maven.plugins</groupId>
        <artifactId>maven-install-plugin</artifactId>
        <version>3.1.2</version>
        <executions>
          <execution>
            <id>install-main</id>
            <phase>validate</phase>
            <goals><goal>install-file</goal></goals>
            <configuration>
              <file>$JAR</file>
              <pomFile>$POM</pomFile>
            </configuration>
          </execution>
          <execution>
            <id>install-native</id>
            <phase>validate</phase>
            <goals><goal>install-file</goal></goals>
            <configuration>
              <file>$NATIVE_JAR</file>
              <groupId>$GROUP_ID</groupId>
              <artifactId>$ARTIFACT_ID</artifactId>
              <version>$VERSION</version>
              <classifier>$NATIVE_CLASSIFIER</classifier>
              <packaging>jar</packaging>
            </configuration>
          </execution>
        </executions>
      </plugin>
    </plugins>
  </build>
</project>
EOF

"$MVN" --batch-mode -q -f "$INSTALLER_DIR/pom.xml" -Dmaven.repo.local="$LOCAL_REPO" validate

PROJECT_DIR="$WORKDIR/probe"
mkdir -p "$PROJECT_DIR/src/main/java/smoketest"

cat > "$PROJECT_DIR/pom.xml" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
  <modelVersion>4.0.0</modelVersion>
  <groupId>arena.smoketest</groupId>
  <artifactId>maven-smoke-test</artifactId>
  <version>0.0.0</version>
  <packaging>jar</packaging>
  <properties>
    <maven.compiler.release>21</maven.compiler.release>
    <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
  </properties>
  <dependencies>
    <dependency>
      <groupId>$GROUP_ID</groupId>
      <artifactId>$ARTIFACT_ID</artifactId>
      <version>$VERSION</version>
    </dependency>
    <dependency>
      <groupId>$GROUP_ID</groupId>
      <artifactId>$ARTIFACT_ID</artifactId>
      <version>$VERSION</version>
      <classifier>$NATIVE_CLASSIFIER</classifier>
    </dependency>
  </dependencies>
</project>
EOF

cat > "$PROJECT_DIR/src/main/java/smoketest/SmokeTest.java" <<'JAVAEOF'
package smoketest;

import arena.junit.ClosedArena;
import arena.junit.OpenArena;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;

import java.util.List;

public final class SmokeTest {
    public static void main(String[] args) throws Exception {
        Match match = new MatchBuilder("maven-smoke-test-match").build();
        ClosedArena closed = new ClosedArena("maven-smoke-test-arena", List.of(match));
        OpenArena arena = closed.open();
        if (arena == null) {
            System.err.println("smoke test: arena is null after open");
            System.exit(1);
        }
        System.out.println("SMOKE_TEST_ARENA_OPENED");
        arena.close();
        System.out.println("SMOKE_TEST_ARENA_CLOSED");
    }
}
JAVAEOF

cd "$PROJECT_DIR"

set +e
OUTPUT="$("$MVN" --batch-mode -q -Dmaven.repo.local="$LOCAL_REPO" -Djna.nosys=true \
  compile org.codehaus.mojo:exec-maven-plugin:3.5.0:java -Dexec.mainClass=smoketest.SmokeTest 2>&1)"
STATUS=$?
set -e
echo "$OUTPUT"

if [ "$STATUS" -ne 0 ]; then
  echo "smoke test FAILED: probe exited $STATUS" >&2
  exit 1
fi
if ! grep -q "SMOKE_TEST_ARENA_OPENED" <<<"$OUTPUT" || ! grep -q "SMOKE_TEST_ARENA_CLOSED" <<<"$OUTPUT"; then
  echo "smoke test FAILED: did not see expected open/close markers in output" >&2
  exit 1
fi

echo "smoke test PASSED: $GROUP_ID:$ARTIFACT_ID:$VERSION (classifier=$NATIVE_CLASSIFIER) installed via mvn, opened and closed a real arena"
