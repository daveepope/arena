package arena.junit.readiness;

public final class TcpReadinessCheck implements ReadinessCheck {
  public static TcpReadinessCheck create() {
    return new TcpReadinessCheck();
  }

  private TcpReadinessCheck() {}
}
