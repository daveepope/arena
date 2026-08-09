package arena.junit.match;

import arena.junit.playbook.ManagedPlaybook;
import arena.junit.playbook.Playbook;
import arena.junit.playbook.PlaybookRegistration;
import arena.junit.playbook.UnmanagedPlaybook;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public final class MatchBuilder {
  private final String name;
  private String network;
  private final List<ArenaRunnableDependency> dependencies = new ArrayList<>();
  private final List<ArenaRunnableComponent> components = new ArrayList<>();
  private final Map<Class<? extends Playbook>, RegisteredPlaybook> playbooks =
      new LinkedHashMap<>();

  public MatchBuilder(String name) {
    this.name = name;
  }

  public MatchBuilder withNetwork(String network) {
    this.network = network;
    return this;
  }

  public MatchBuilder addDependency(ArenaRunnableDependency dependency) {
    dependencies.add(dependency);
    return this;
  }

  public MatchBuilder addComponent(ArenaRunnableComponent component) {
    components.add(component);
    return this;
  }

  public MatchBuilder registerPlaybook(Playbook playbook) {
    return registerPlaybook(playbook, true);
  }

  public MatchBuilder registerPlaybook(Playbook playbook, boolean execOnDependencyStart) {
    if (playbook == null) {
      throw new IllegalArgumentException("playbook must not be null");
    }
    if (!(playbook instanceof ManagedPlaybook) && !(playbook instanceof UnmanagedPlaybook)) {
      throw new IllegalArgumentException(
          "playbook "
              + playbook.getClass().getName()
              + " must implement ManagedPlaybook or UnmanagedPlaybook to be registered on a match");
    }
    if (execOnDependencyStart && !(playbook instanceof PlaybookRegistration)) {
      throw new IllegalArgumentException(
          "playbook "
              + playbook.getClass().getName()
              + " must implement PlaybookRegistration to be registered with"
              + " execOnDependencyStart=true");
    }
    Class<? extends Playbook> klass = playbook.getClass();
    if (playbooks.containsKey(klass)) {
      throw new IllegalStateException(
          "playbook class already registered on match '" + name + "': " + klass.getName());
    }
    playbooks.put(klass, new RegisteredPlaybook(playbook, execOnDependencyStart));
    return this;
  }

  public Match build() {
    return new Match(name, dependencies, components, network, playbooks);
  }
}
