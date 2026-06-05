package arena.junit.playbook;

import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class HttpHeaderPattern {
  private HttpHeaderPattern() {}

  public static ObjectNode equalTo(String value) {
    ObjectNode n = ArenaJson.object();
    n.put("equal_to", value);
    return n;
  }

  public static ObjectNode matching(String regex) {
    ObjectNode n = ArenaJson.object();
    n.put("matches", regex);
    return n;
  }
}
