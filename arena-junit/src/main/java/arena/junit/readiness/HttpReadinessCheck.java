package arena.junit.readiness;
import java.net.HttpURLConnection;
import java.net.URI;
import java.time.Instant;

public final class HttpReadinessCheck implements ReadinessCheck {
  public static HttpReadinessCheck create() {
    return new HttpReadinessCheck();
  }

  private HttpReadinessCheck() {}

  @Override
  public void awaitReady(String identifier, String target, int timeoutMs) throws Exception {
    long deadline = Instant.now().toEpochMilli() + timeoutMs;
    Exception last = null;
    while (Instant.now().toEpochMilli() < deadline) {
      try {
        HttpURLConnection c = (HttpURLConnection) URI.create(target).toURL().openConnection();
        c.setRequestMethod("GET");
        c.setConnectTimeout(500);
        c.setReadTimeout(500);
        int code = c.getResponseCode();
        if (code >= 200 && code < 300) {
          return;
        }
      } catch (Exception e) {
        last = e;
      }
      Thread.sleep(100);
    }
    if (last != null) {
      throw last;
    }
    throw new IllegalStateException("readiness timeout for " + target);
  }
}
