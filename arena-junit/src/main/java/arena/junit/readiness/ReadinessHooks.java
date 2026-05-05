package arena.junit.readiness;
public final class ReadinessHooks {
  public record Hook(String identifier, String target, ReadinessCheck check) {}

  private ReadinessHooks() {}
}
