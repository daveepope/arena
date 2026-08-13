package arena.junit.dep;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.match.ArenaRunnableDependency;
import arena.junit.playbook.LocalstackModels;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.List;
import org.junit.jupiter.api.Test;

final class LocalstackDependencyBuilderSerializationTest {

  static final class StubDependency implements ArenaRunnableDependency {
    private final String identifier;

    StubDependency(String identifier) {
      this.identifier = identifier;
    }

    @Override
    public ObjectNode forFfi() {
      ObjectNode n = ArenaJson.object();
      n.put("identifier", identifier);
      return n;
    }
  }

  @Test
  void build_minimalName_serializesTypeAndEmptyCollections() {
    ObjectNode config = new LocalstackDependencyBuilder("ls").build().forFfi();
    assertEquals("localstack", config.path("type").asText());
    assertTrue(config.path("identifier").asText().startsWith("arena-localstack-ls-"));
    assertTrue(config.path("services").isEmpty());
    assertTrue(config.path("queues").isEmpty());
    assertTrue(config.path("lambdas").isEmpty());
    assertTrue(config.path("event_buses").isEmpty());
    assertTrue(config.path("event_rules").isEmpty());
    assertFalse(config.has("children"));
  }

  @Test
  void withPortImageAndContainerName_setsScalarFields() {
    ObjectNode config =
        new LocalstackDependencyBuilder("ls")
            .withPort(4567)
            .withImageName("localstack/localstack")
            .withImageTag("3.0")
            .withContainerName("ls-box")
            .build()
            .forFfi();
    assertEquals(4567, config.path("port").asInt());
    assertEquals("localstack/localstack", config.path("image_name").asText());
    assertEquals("3.0", config.path("image_tag").asText());
    assertEquals("ls-box", config.path("container_name").asText());
  }

  @Test
  void withServices_singleAndBulk_appendsToServicesArray() {
    ObjectNode config =
        new LocalstackDependencyBuilder("ls")
            .withService("sqs")
            .withServices(List.of("sns", "lambda"))
            .build()
            .forFfi();
    ArrayNode services = (ArrayNode) config.path("services");
    assertEquals(3, services.size());
    assertEquals("sqs", services.get(0).asText());
    assertEquals("sns", services.get(1).asText());
    assertEquals("lambda", services.get(2).asText());
  }

  @Test
  void withQueueAndFifoQueue_appendsQueueSpecs() {
    ObjectNode config =
        new LocalstackDependencyBuilder("ls")
            .withQueue("plain-queue")
            .withFifoQueue("ordered-queue")
            .build()
            .forFfi();
    ArrayNode queues = (ArrayNode) config.path("queues");
    assertEquals(2, queues.size());
    assertEquals("plain-queue", queues.get(0).path("name").asText());
    assertFalse(queues.get(0).path("fifo").asBoolean());
    assertEquals("ordered-queue", queues.get(1).path("name").asText());
    assertTrue(queues.get(1).path("fifo").asBoolean());
  }

  @Test
  void withLambda_absoluteSourceDirAndEnvironment_serializesLambdaSpec() {
    LocalstackModels.LambdaSpec spec =
        new LocalstackModels.LambdaSpec(
            "fn",
            "python3.12",
            "handler.main",
            ".",
            List.of(new LocalstackModels.EnvPair("KEY", "VALUE")));
    ObjectNode config = new LocalstackDependencyBuilder("ls").withLambda(spec).build().forFfi();
    ObjectNode lambda = (ObjectNode) config.path("lambdas").get(0);
    assertEquals("fn", lambda.path("name").asText());
    assertEquals("python3.12", lambda.path("runtime").asText());
    assertEquals("handler.main", lambda.path("handler").asText());
    assertEquals(
        java.nio.file.Path.of(".").toAbsolutePath().normalize().toString(),
        lambda.path("source_dir").asText());
    ArrayNode env = (ArrayNode) lambda.path("environment");
    assertEquals("KEY", env.get(0).get(0).asText());
    assertEquals("VALUE", env.get(0).get(1).asText());
  }

  @Test
  void withLambda_defaultEnvironmentOverload_serializesEmptyEnvironment() {
    LocalstackModels.LambdaSpec spec =
        new LocalstackModels.LambdaSpec("fn", "python3.12", "handler.main", ".");
    ObjectNode config = new LocalstackDependencyBuilder("ls").withLambda(spec).build().forFfi();
    ArrayNode env = (ArrayNode) config.path("lambdas").get(0).path("environment");
    assertTrue(env.isEmpty());
  }

  @Test
  void withEventBus_appendsEventBusSpec() {
    ObjectNode config = new LocalstackDependencyBuilder("ls").withEventBus("orders").build().forFfi();
    assertEquals("orders", config.path("event_buses").get(0).path("name").asText());
  }

  @Test
  void withEventRule_sqsTarget_serializesSqsQueueTargetKind() {
    LocalstackModels.EventRuleSpec spec =
        new LocalstackModels.EventRuleSpec(
            "rule",
            "{\"source\":[\"custom\"]}",
            List.of(
                new LocalstackModels.EventRuleTarget(
                    "t1", new LocalstackModels.SqsQueueTarget("my-queue"))));
    ObjectNode config = new LocalstackDependencyBuilder("ls").withEventRule(spec).build().forFfi();
    ObjectNode rule = (ObjectNode) config.path("event_rules").get(0);
    assertEquals("rule", rule.path("name").asText());
    assertFalse(rule.has("event_bus"));
    ObjectNode target = (ObjectNode) rule.path("targets").get(0);
    assertEquals("t1", target.path("target_id").asText());
    assertEquals("sqs_queue", target.path("kind").asText());
    assertEquals("my-queue", target.path("queue_name").asText());
  }

  @Test
  void withEventRule_lambdaTargetWithEventBus_serializesLambdaTargetKind() {
    LocalstackModels.EventRuleSpec spec =
        new LocalstackModels.EventRuleSpec(
            "rule",
            "{\"source\":[\"custom\"]}",
            List.of(
                new LocalstackModels.EventRuleTarget(
                    "t1", new LocalstackModels.LambdaTarget("my-fn"))),
            "orders");
    ObjectNode config = new LocalstackDependencyBuilder("ls").withEventRule(spec).build().forFfi();
    ObjectNode rule = (ObjectNode) config.path("event_rules").get(0);
    assertEquals("orders", rule.path("event_bus").asText());
    ObjectNode target = (ObjectNode) rule.path("targets").get(0);
    assertEquals("lambda", target.path("kind").asText());
    assertEquals("my-fn", target.path("function_name").asText());
  }

  @Test
  void addChildDependency_nonEmptyChildren_serializesChildrenArray() {
    ObjectNode config =
        new LocalstackDependencyBuilder("ls")
            .addChildDependency(new StubDependency("child-a"))
            .build()
            .forFfi();
    ArrayNode children = (ArrayNode) config.path("children");
    assertEquals(1, children.size());
    assertEquals("child-a", children.get(0).path("identifier").asText());
  }

  @Test
  void identifier_returnsConfiguredIdentifier() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").build();
    assertTrue(dep.identifier().startsWith("arena-localstack-ls-"));
  }

  @Test
  void port_noExplicitPort_returnsInternalPortDefault() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").build();
    assertEquals(LocalstackModels.LOCALSTACK_INTERNAL_DOCKER_PORT, dep.port());
  }

  @Test
  void port_explicitPort_returnsConfiguredPort() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").withPort(4599).build();
    assertEquals(4599, dep.port());
  }

  @Test
  void endpointUrl_defaultHost_usesLocalhost() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").withPort(4566).build();
    assertEquals("http://localhost:4566", dep.endpointUrl());
  }

  @Test
  void endpointUrl_explicitHost_usesGivenHost() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").withPort(4566).build();
    assertEquals("http://ls-host:4566", dep.endpointUrl("ls-host"));
  }

  @Test
  void internalEndpointUrl_noContainerName_usesIdentifierAsHost() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").build();
    assertEquals(
        "http://" + dep.identifier() + ":" + LocalstackModels.LOCALSTACK_INTERNAL_DOCKER_PORT,
        dep.internalEndpointUrl());
  }

  @Test
  void internalEndpointUrl_explicitContainerName_usesContainerNameAsHost() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").build();
    assertEquals(
        "http://ls-box:" + LocalstackModels.LOCALSTACK_INTERNAL_DOCKER_PORT,
        dep.internalEndpointUrl("ls-box"));
  }

  @Test
  void queueUrl_defaultHostAndAccount_buildsLocalstackStyleUrl() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").withPort(4566).build();
    assertEquals(
        "http://localhost:4566/000000000000/my-queue", dep.queueUrl("my-queue"));
  }

  @Test
  void queueUrl_explicitHost_usesGivenHost() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").withPort(4566).build();
    assertEquals(
        "http://ls-host:4566/000000000000/my-queue", dep.queueUrl("my-queue", "ls-host"));
  }

  @Test
  void queueUrl_explicitHostAndAccount_usesBothOverrides() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").withPort(4566).build();
    assertEquals(
        "http://ls-host:4566/111111111111/my-queue",
        dep.queueUrl("my-queue", "ls-host", "111111111111"));
  }

  @Test
  void queueArn_defaultRegionAndAccount_buildsArn() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").build();
    assertEquals(
        "arn:aws:sqs:us-east-1:000000000000:my-queue", dep.queueArn("my-queue"));
  }

  @Test
  void queueArn_explicitRegion_usesGivenRegion() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").build();
    assertEquals(
        "arn:aws:sqs:eu-west-1:000000000000:my-queue", dep.queueArn("my-queue", "eu-west-1"));
  }

  @Test
  void queueArn_explicitRegionAndAccount_usesBothOverrides() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").build();
    assertEquals(
        "arn:aws:sqs:eu-west-1:222222222222:my-queue",
        dep.queueArn("my-queue", "eu-west-1", "222222222222"));
  }

  @Test
  void lambdaArn_defaultRegionAndAccount_buildsArn() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").build();
    assertEquals(
        "arn:aws:lambda:us-east-1:000000000000:function:my-fn", dep.lambdaArn("my-fn"));
  }

  @Test
  void lambdaArn_explicitRegion_usesGivenRegion() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").build();
    assertEquals(
        "arn:aws:lambda:eu-west-1:000000000000:function:my-fn",
        dep.lambdaArn("my-fn", "eu-west-1"));
  }

  @Test
  void lambdaArn_explicitRegionAndAccount_usesBothOverrides() {
    LocalstackDependency dep = new LocalstackDependencyBuilder("ls").build();
    assertEquals(
        "arn:aws:lambda:eu-west-1:333333333333:function:my-fn",
        dep.lambdaArn("my-fn", "eu-west-1", "333333333333"));
  }
}
