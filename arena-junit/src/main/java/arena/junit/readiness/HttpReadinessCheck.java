package arena.junit.readiness;

public final class HttpReadinessCheck implements ReadinessCheck {
  public static HttpReadinessCheck create() {
    return new HttpReadinessCheck();
  }

  private HttpReadinessCheck() {}
}
