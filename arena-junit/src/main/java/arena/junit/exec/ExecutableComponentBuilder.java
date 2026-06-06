package arena.junit.exec;
import arena.junit.readiness.ReadinessCheck;
import arena.junit.readiness.ReadinessChecksFfi;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class ExecutableComponentBuilder {
  private final ObjectNode config;
  private final List<ReadinessChecksFfi.ReadinessEntry> readiness = new ArrayList<>();

  public ExecutableComponentBuilder(String name) {
    ObjectNode c = ArenaJson.object();
    c.put("type", "exec");
    c.put("identifier", ArenaIdentifiers.build("arena-executable-component", name));
    c.set("env_vars", ArenaJson.object());
    c.set("runtime_args", ArenaJson.array());
    this.config = c;
  }

  public ExecutableComponentBuilder withExecutablePath(String path) {
    config.put("executable_path", path);
    return this;
  }

  public ExecutableComponentBuilder withSourcePath(String path) {
    config.put("source_path", path);
    return this;
  }

  public ExecutableComponentBuilder withBuildTool(BuildTool tool) {
    config.put("build_tool", tool.value());
    return this;
  }

  public ExecutableComponentBuilder withBuildToolCustom(String command, List<String> args) {
    config.set("build_tool", BuildTool.customBuild(command, args));
    return this;
  }

  public ExecutableComponentBuilder withEnvVar(String key, String value) {
    ((ObjectNode) config.get("env_vars")).put(key, value);
    return this;
  }

  public ExecutableComponentBuilder withRuntimeArg(String name, String value) {
    ArrayNode use = (ArrayNode) config.get("runtime_args");
    ObjectNode pair = ArenaJson.object();
    pair.put("name", name);
    pair.put("value", value);
    use.add(pair);
    return this;
  }

  public ExecutableComponentBuilder withReadinessCheck(ReadinessCheck check, String target) {
    return withReadinessCheck(check, target, 10_000L);
  }

  public ExecutableComponentBuilder withReadinessCheck(
      ReadinessCheck check, String target, long timeoutMs) {
    readiness.add(new ReadinessChecksFfi.ReadinessEntry(check, target, timeoutMs));
    return this;
  }

  public ExecutableComponent build() {
    return new ExecutableComponent(config.deepCopy(), readiness);
  }
}
