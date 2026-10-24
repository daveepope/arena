package arena.junit.ffi;

import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.support.ArenaJson;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.InputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

final class ArenaDashboardBindingsComponentTest {

  private HttpServer captureServer;
  private final List<String> capturedBodies = new CopyOnWriteArrayList<>();

  private String startCaptureServer() throws IOException {
    captureServer = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
    captureServer.createContext(
        "/",
        exchange -> {
          try (InputStream body = exchange.getRequestBody()) {
            capturedBodies.add(new String(body.readAllBytes(), StandardCharsets.UTF_8));
          }
          exchange.sendResponseHeaders(200, -1);
          exchange.close();
        });
    captureServer.start();
    return "http://127.0.0.1:" + captureServer.getAddress().getPort();
  }

  @AfterEach
  void stopCaptureServer() {
    if (captureServer != null) {
      captureServer.stop(0);
      captureServer = null;
    }
  }

  @Test
  void registerDashboardObserver_validUrl_returnsRemovableToken() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    long token = ArenaBindings.registerDashboardObserver("http://127.0.0.1:1");

    assertNotEquals(0L, token);
    ArenaBindings.removeLifecycleObserver(token);
  }

  @Test
  void registerDashboardObserver_nullUrl_throwsArenaBindingError() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    assertThrows(ArenaBindingError.class, () -> ArenaBindings.registerDashboardObserver(null));
  }

  @Test
  void dashboardEnsureRunning_noServerAndNoSearchDir_throwsArenaBindingError() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    assertThrows(ArenaBindingError.class, () -> ArenaBindings.dashboardEnsureRunning(1, null));
  }

  @Test
  void dashboardEnsureRunning_negativePort_throwsArenaBindingError() {
    assertThrows(ArenaBindingError.class, () -> ArenaBindings.dashboardEnsureRunning(-1, null));
  }

  @Test
  void dashboardEnsureRunning_portAboveRange_throwsArenaBindingError() {
    assertThrows(ArenaBindingError.class, () -> ArenaBindings.dashboardEnsureRunning(65536, null));
  }

  @Test
  void registerDashboardObserver_openAndClose_forwardsStateViaHttp() throws Exception {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    String url = startCaptureServer();

    long token = ArenaBindings.registerDashboardObserver(url);
    try {
      var handle = ArenaBindings.arenaOpen("junit-dashboard-forward", "{}", ArenaLogLevel.INFO);
      ArenaBindings.arenaClose(handle);
    } finally {
      ArenaBindings.removeLifecycleObserver(token);
    }

    assertTrue(documentIds().contains("junit-dashboard-forward"), "no states were forwarded via http");
  }

  private List<String> documentIds() throws IOException {
    List<String> ids = new java.util.ArrayList<>();
    for (String body : capturedBodies) {
      ids.add(ArenaJson.MAPPER.readTree(body).path("id").asText());
    }
    return ids;
  }
}
