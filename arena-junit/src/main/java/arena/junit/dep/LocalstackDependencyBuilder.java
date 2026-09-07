package arena.junit.dep;
import arena.junit.match.ArenaRunnableDependency;
import arena.junit.playbook.LocalstackModels;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;
import arena.junit.support.ChildrenFfi;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class LocalstackDependencyBuilder {
  private final ObjectNode config;
  private final List<ArenaRunnableDependency> children = new ArrayList<>();

  public LocalstackDependencyBuilder(String name) {
    ObjectNode c = ArenaJson.object();
    c.put("type", "localstack");
    c.put("identifier", ArenaIdentifiers.build("arena-localstack", name));
    c.set("services", ArenaJson.array());
    c.set("queues", ArenaJson.array());
    c.set("lambdas", ArenaJson.array());
    c.set("event_buses", ArenaJson.array());
    c.set("event_rules", ArenaJson.array());
    this.config = c;
  }

  public LocalstackDependencyBuilder withExpiry(java.time.Duration expiry) {
    config.put("expiry_seconds", expirySeconds(expiry));
    return this;
  }

  public LocalstackDependencyBuilder withoutExpiry() {
    config.put("expiry_seconds", 0);
    return this;
  }

  public LocalstackDependencyBuilder withPort(int port) {
    config.put("port", port);
    return this;
  }

  public LocalstackDependencyBuilder withImageName(String imageName) {
    config.put("image_name", imageName);
    return this;
  }

  public LocalstackDependencyBuilder withImageTag(String imageTag) {
    config.put("image_tag", imageTag);
    return this;
  }

  public LocalstackDependencyBuilder withContainerName(String name) {
    config.put("container_name", name);
    return this;
  }

  public LocalstackDependencyBuilder withService(String service) {
    ((ArrayNode) config.get("services")).add(service);
    return this;
  }

  public LocalstackDependencyBuilder withServices(List<String> services) {
    for (String s : services) {
      withService(s);
    }
    return this;
  }

  public LocalstackDependencyBuilder withQueue(String name) {
    return withQueueSpec(new LocalstackModels.QueueSpec(name, false));
  }

  public LocalstackDependencyBuilder withFifoQueue(String name) {
    return withQueueSpec(new LocalstackModels.QueueSpec(name, true));
  }

  public LocalstackDependencyBuilder withQueueSpec(LocalstackModels.QueueSpec spec) {
    ObjectNode q = ArenaJson.object();
    q.put("name", spec.name());
    q.put("fifo", spec.fifo());
    ((ArrayNode) config.get("queues")).add(q);
    return this;
  }

  public LocalstackDependencyBuilder withLambda(LocalstackModels.LambdaSpec spec) {
    java.nio.file.Path p = java.nio.file.Path.of(spec.sourceDir()).toAbsolutePath().normalize();
    ObjectNode lam = ArenaJson.object();
    lam.put("name", spec.name());
    lam.put("runtime", spec.runtime());
    lam.put("handler", spec.handler());
    lam.put("source_dir", p.toString());
    ArrayNode env = ArenaJson.array();
    for (LocalstackModels.EnvPair e : spec.environment()) {
      ArrayNode pair = ArenaJson.array();
      pair.add(e.key());
      pair.add(e.value());
      env.add(pair);
    }
    lam.set("environment", env);
    ((ArrayNode) config.get("lambdas")).add(lam);
    return this;
  }

  public LocalstackDependencyBuilder withEventBus(String name) {
    ObjectNode b = ArenaJson.object();
    b.put("name", name);
    ((ArrayNode) config.get("event_buses")).add(b);
    return this;
  }

  public LocalstackDependencyBuilder withEventRule(LocalstackModels.EventRuleSpec spec) {
    ObjectNode rule = ArenaJson.object();
    rule.put("name", spec.name());
    if (spec.eventBus() != null) {
      rule.put("event_bus", spec.eventBus());
    }
    rule.put("event_pattern", spec.eventPattern());
    ArrayNode targets = ArenaJson.array();
    for (LocalstackModels.EventRuleTarget t : spec.targets()) {
      ObjectNode tn = ArenaJson.object();
      tn.put("target_id", t.targetId());
      tn.setAll(targetKindJson(t.kind()));
      targets.add(tn);
    }
    rule.set("targets", targets);
    ((ArrayNode) config.get("event_rules")).add(rule);
    return this;
  }

  private static ObjectNode targetKindJson(LocalstackModels.EventTargetKind kind) {
    ObjectNode n = ArenaJson.object();
    if (kind instanceof LocalstackModels.SqsQueueTarget sq) {
      n.put("kind", "sqs_queue");
      n.put("queue_name", sq.queueName());
    } else if (kind instanceof LocalstackModels.LambdaTarget lm) {
      n.put("kind", "lambda");
      n.put("function_name", lm.functionName());
    } else {
      throw new IllegalArgumentException("unsupported target kind");
    }
    return n;
  }

  public LocalstackDependencyBuilder addChildDependency(ArenaRunnableDependency child) {
    this.children.add(child);
    return this;
  }

  public LocalstackDependency build() {
    ObjectNode cfg = config.deepCopy();
    if (!children.isEmpty()) {
      cfg.set("children", ChildrenFfi.buildDependencies(children));
    }
    return new LocalstackDependency(cfg);
  }

  private static long expirySeconds(java.time.Duration expiry) {
    if (expiry.isNegative()) {
      throw new IllegalArgumentException("expiry must not be negative: " + expiry);
    }
    long seconds = expiry.toSeconds();
    return seconds == 0 && !expiry.isZero() ? 1 : seconds;
  }

}
