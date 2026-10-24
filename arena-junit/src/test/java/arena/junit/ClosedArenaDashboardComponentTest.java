package arena.junit;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.support.ArenaJson;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.InputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

final class ClosedArenaDashboardComponentTest {

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

  private static ClosedArena plainClosedArena(String name) {
    Match match = new MatchBuilder(name + "-match").build();
    return new ClosedArena(name, List.of(match));
  }

  @Test
  void withLifecycleDashboard_openAndClose_forwardsStateViaHttp() throws Exception {
    String url = startCaptureServer();

    OpenArena opened = plainClosedArena("junit-visualize-forward").withLifecycleDashboard(url).open();
    opened.close();

    assertTrue(documentIds().contains("junit-visualize-forward"), "no states were forwarded via http");
  }

  @Test
  void withLifecycleDashboard_dashboardUnavailable_opensArenaAnyway() throws Exception {
    OpenArena opened =
        plainClosedArena("junit-visualize-unavailable").withLifecycleDashboard(1, null).open();
    opened.close();
  }

  @Test
  void withLifecycleDashboard_afterClose_unregistersForwarding() throws Exception {
    String url = startCaptureServer();

    OpenArena opened = plainClosedArena("junit-visualize-teardown").withLifecycleDashboard(url).open();
    opened.close();

    plainClosedArena("junit-visualize-teardown-unrelated").open().close();

    assertFalse(
        documentIds().contains("junit-visualize-teardown-unrelated"),
        "forwarding kept running after close");
  }

  private List<String> documentIds() throws IOException {
    List<String> ids = new java.util.ArrayList<>();
    for (String body : capturedBodies) {
      ids.add(ArenaJson.MAPPER.readTree(body).path("id").asText());
    }
    return ids;
  }
}
