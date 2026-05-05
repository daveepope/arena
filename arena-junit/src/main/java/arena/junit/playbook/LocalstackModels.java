package arena.junit.playbook;
public final class LocalstackModels {
  public static final int LOCALSTACK_INTERNAL_DOCKER_PORT = 4566;
  public static final String LOCALSTACK_DEFAULT_ACCOUNT_ID = "000000000000";
  public static final String LOCALSTACK_DEFAULT_REGION = "us-east-1";

  public record QueueSpec(String name, boolean fifo) {}

  public record LambdaSpec(
      String name,
      String runtime,
      String handler,
      String sourceDir,
      java.util.List<EnvPair> environment) {
    public LambdaSpec(String name, String runtime, String handler, String sourceDir) {
      this(name, runtime, handler, sourceDir, java.util.List.of());
    }
  }

  public record EnvPair(String key, String value) {}

  public record EventBusSpec(String name) {}

  public record SqsQueueTarget(String queueName) implements EventTargetKind {}

  public record LambdaTarget(String functionName) implements EventTargetKind {}

  public sealed interface EventTargetKind permits SqsQueueTarget, LambdaTarget {}

  public record EventRuleTarget(String targetId, EventTargetKind kind) {}

  public record EventRuleSpec(
      String name, String eventPattern, java.util.List<EventRuleTarget> targets, String eventBus) {
    public EventRuleSpec(
        String name, String eventPattern, java.util.List<EventRuleTarget> targets) {
      this(name, eventPattern, targets, null);
    }
  }

  private LocalstackModels() {}
}
