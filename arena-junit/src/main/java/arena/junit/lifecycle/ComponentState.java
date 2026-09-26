package arena.junit.lifecycle;

import java.util.List;

public final class ComponentState {
  public String id = "";
  public String state = "";
  public List<Fault> faults = List.of();
  public List<ComponentState> children = List.of();

  public ComponentState find(String identifier) {
    if (id.equals(identifier)) {
      return this;
    }
    for (ComponentState child : children) {
      ComponentState found = child.find(identifier);
      if (found != null) {
        return found;
      }
    }
    return null;
  }
}
