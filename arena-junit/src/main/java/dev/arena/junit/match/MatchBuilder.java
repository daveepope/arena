package dev.arena.junit.match;
import dev.arena.junit.playbook.ArenaPlaybookRegistration;

import java.util.ArrayList;
import java.util.List;

public final class MatchBuilder {
  private final String name;
  private String network;
  private final List<ArenaMatchPiece> dependencies = new ArrayList<>();
  private final List<ArenaMatchPiece> components = new ArrayList<>();
  private final List<RegisteredPlaybook> playbooks = new ArrayList<>();

  public MatchBuilder(String name) {
    this.name = name;
  }

  public MatchBuilder withNetwork(String network) {
    this.network = network;
    return this;
  }

  public MatchBuilder addDependency(ArenaMatchPiece dependency) {
    dependencies.add(dependency);
    return this;
  }

  public MatchBuilder addComponent(ArenaMatchPiece component) {
    components.add(component);
    return this;
  }

  public MatchBuilder registerPlaybook(ArenaPlaybookRegistration playbook) {
    return registerPlaybook(playbook, true);
  }

  public MatchBuilder registerPlaybook(ArenaPlaybookRegistration playbook, boolean execOnDependencyStart) {
    playbooks.add(new RegisteredPlaybook(playbook, execOnDependencyStart));
    return this;
  }

  public Match build() {
    return new Match(name, dependencies, components, network, playbooks);
  }
}
