package arena.junit.lifecycle;

import arena.junit.support.ArenaJson;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectReader;
import java.util.List;

public final class ArenaState {
  public static final String ARENA_FAULTED = "arena_faulted";
  public static final String ARENA_CLOSED = "arena_closed";

  private static final ObjectReader READER =
      ArenaJson.MAPPER
          .readerFor(ArenaState.class)
          .without(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES);

  public String id = "";
  public String state = "";
  public String at = "";
  public List<DependencyState> dependencies = List.of();
  public List<ComponentState> components = List.of();
  public List<Fault> faults = List.of();

  public static ArenaState parse(String document) {
    try {
      ArenaState parsed = READER.readValue(document);
      if (parsed == null) {
        throw new IllegalArgumentException("arena state document must be a json object");
      }
      return parsed;
    } catch (java.io.IOException e) {
      throw new IllegalArgumentException("arena state document failed to parse", e);
    }
  }

  public boolean isFaulted() {
    return ARENA_FAULTED.equals(state);
  }

  public DependencyState dependency(String identifier) {
    for (DependencyState dep : dependencies) {
      DependencyState found = dep.find(identifier);
      if (found != null) {
        return found;
      }
    }
    return null;
  }

  public ComponentState component(String identifier) {
    for (ComponentState comp : components) {
      ComponentState found = comp.find(identifier);
      if (found != null) {
        return found;
      }
    }
    return null;
  }
}
