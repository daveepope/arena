package arena.junit.exec;
import arena.junit.readiness.ReadinessCheck;
import arena.junit.readiness.ReadinessChecksFfi;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class ContainerizedComponentBuilder {
  private final ObjectNode config;
  private final List<ReadinessChecksFfi.ReadinessEntry> readiness = new ArrayList<>();

  public ContainerizedComponentBuilder(String name, String containerfile) {
    config =
        ArenaJson.object()
            .put("type", "container")
            .put("identifier", ArenaIdentifiers.build("arena-containerized-component", name))
            .put("containerfile", containerfile);
    config.set("env_vars", ArenaJson.object());
    config.set("runtime_args", ArenaJson.array());
    config.set("port_mappings", ArenaJson.array());
    config.set("host_mappings", ArenaJson.array());
  }

  public ContainerizedComponentBuilder withBuildContext(String path) {
    config.put("build_context", path);
    return this;
  }

  public ContainerizedComponentBuilder withImageTag(String tag) {
    config.put("image_tag", tag);
    return this;
  }

  public ContainerizedComponentBuilder withNetwork(String network) {
    config.put("network", network);
    return this;
  }

  public ContainerizedComponentBuilder withPortMapping(int hostPort, int containerPort) {
    ArrayNode arr = (ArrayNode) config.get("port_mappings");
    ObjectNode m = ArenaJson.object();
    m.put("host_port", hostPort);
    m.put("container_port", containerPort);
    arr.add(m);
    return this;
  }

  public ContainerizedComponentBuilder withHostMapping(String hostMapping) {
    ((ArrayNode) config.get("host_mappings")).add(hostMapping);
    return this;
  }

  public ContainerizedComponentBuilder withEnvVar(String key, String value) {
    ((ObjectNode) config.get("env_vars")).put(key, value);
    return this;
  }

  public ContainerizedComponentBuilder withRuntimeArg(String name, String value) {
    ArrayNode arr = (ArrayNode) config.get("runtime_args");
    ObjectNode pair = ArenaJson.object();
    pair.put("name", name);
    pair.put("value", value);
    arr.add(pair);
    return this;
  }

  public ContainerizedComponentBuilder withReadinessCheck(ReadinessCheck check, String target) {
    return withReadinessCheck(check, target, 10_000L);
  }

  public ContainerizedComponentBuilder withReadinessCheck(
      ReadinessCheck check, String target, long timeoutMs) {
    readiness.add(new ReadinessChecksFfi.ReadinessEntry(check, target, timeoutMs));
    return this;
  }

  public ContainerizedComponent build() {
    return new ContainerizedComponent(config.deepCopy(), readiness);
  }
}
