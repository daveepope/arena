package arena.junit.exec;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.List;

public enum BuildTool {
  CARGO("cargo"),
  MAVEN("maven"),
  GRADLE("gradle"),
  DOTNET("dotnet"),
  MAKE("make"),
  CMAKE("cmake"),
  PYTHON("python");

  private final String value;

  BuildTool(String value) {
    this.value = value;
  }

  public String value() {
    return value;
  }

  public static ObjectNode customBuild(String command, List<String> args) {
    ObjectNode n = ArenaJson.object();
    n.put("command", command);
    ArrayNode a = ArenaJson.array();
    for (String x : args) {
      a.add(x);
    }
    n.set("args", a);
    return n;
  }
}
