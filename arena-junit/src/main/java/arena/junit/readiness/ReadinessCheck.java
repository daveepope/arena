package arena.junit.readiness;
public interface ReadinessCheck {
  void awaitReady(String identifier, String target, int timeoutMs) throws Exception;
}
