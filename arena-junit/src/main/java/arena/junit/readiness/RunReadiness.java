package arena.junit.readiness;
public final class RunReadiness {
  private RunReadiness() {}

  public static void runReadiness(ReadinessCheck check, String identifier, String target, int timeoutMs)
      throws Exception {
    check.awaitReady(identifier, target, timeoutMs);
  }
}
