package arena.junit.lifecycle;

import java.util.List;

public final class DependencyState {
  public String id = "";
  public String state = "";
  public List<Fault> faults = List.of();
  public List<DependencyState> children = List.of();

  public DependencyState find(String identifier) {
    if (id.equals(identifier)) {
      return this;
    }
    for (DependencyState child : children) {
      DependencyState found = child.find(identifier);
      if (found != null) {
        return found;
      }
    }
    return null;
  }
}
