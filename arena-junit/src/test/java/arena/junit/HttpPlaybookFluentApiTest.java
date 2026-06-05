package arena.junit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.readings.testruntime.ReadingsEphemeralTestRuntime;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.playbook.ActiveHttpPlaybook;
import arena.junit.playbook.HttpHeaderPattern;
import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.HttpResponse;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse.BodyHandlers;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.RegisterExtension;

final class HttpPlaybookFluentApiTest {

  private static final ReadingsEphemeralTestRuntime RT = ReadingsEphemeralTestRuntime.get();
  private static String dependencyIdentifier;
  private static int httpPort;

  static final class FluentArenaFixture extends ClosedArenaExtension {
    @Override
    protected ClosedArena buildClosedArena() throws Exception {
      httpPort = RT.calibrationHostPort;
      HttpDependency http = new HttpDependencyBuilder("fluent-http").withPort(httpPort).build();
      dependencyIdentifier = http.identifier();
      Match match = new MatchBuilder("fluent-http-match").addDependency(http).build();
      return new ClosedArena("fluent-http-arena", List.of(match));
    }
  }

  @RegisterExtension static final FluentArenaFixture fluentArena = new FluentArenaFixture();

  @Test
  void httpPlaybookBuilder_sequence_thenReturn_returnsStatusesInOrder() throws Exception {
    OpenArena arena = fluentArena.openArena();
    HttpClient client = HttpClient.newHttpClient();
    try (ActiveHttpPlaybook active =
        new HttpPlaybookBuilder(dependencyIdentifier)
            .get("/api/telemetry/altitude")
            .willReturn(HttpResponse.serverError())
            .thenReturn(HttpResponse.status(503))
            .thenReturn(HttpResponse.okJson(Map.of("altitude_km", 185)))
            .open(arena)) {
      String base = "http://127.0.0.1:" + httpPort;
      assertEquals(
          500,
          client
              .send(
                  HttpRequest.newBuilder().uri(URI.create(base + "/api/telemetry/altitude")).GET().build(),
                  BodyHandlers.discarding())
              .statusCode());
      assertEquals(
          503,
          client
              .send(
                  HttpRequest.newBuilder().uri(URI.create(base + "/api/telemetry/altitude")).GET().build(),
                  BodyHandlers.discarding())
              .statusCode());
      assertEquals(
          200,
          client
              .send(
                  HttpRequest.newBuilder().uri(URI.create(base + "/api/telemetry/altitude")).GET().build(),
                  BodyHandlers.discarding())
              .statusCode());
    }
  }

  @Test
  void httpPlaybookBuilder_scenarioState_returnsExpectedStageBodies() throws Exception {
    OpenArena arena = fluentArena.openArena();
    HttpClient client = HttpClient.newHttpClient();
    String base = "http://127.0.0.1:" + httpPort;
    try (ActiveHttpPlaybook active =
        new HttpPlaybookBuilder(dependencyIdentifier)
            .get("/api/vehicle/telemetry")
            .inScenario("saturn-v-launch")
            .willReturn(HttpResponse.okJson(Map.of("stage", "terminal-count")))
            .post("/api/vehicle/main-engine-start")
            .inScenario("saturn-v-launch")
            .willSetStateTo("first-stage-flight")
            .willReturn(HttpResponse.okJson(Map.of("stage", "main-engine-start")))
            .get("/api/vehicle/telemetry")
            .inScenario("saturn-v-launch")
            .whenStateIs("first-stage-flight")
            .willReturn(HttpResponse.okJson(Map.of("stage", "first-stage-flight")))
            .open(arena)) {
      assertTrue(
          client
              .send(
                  HttpRequest.newBuilder().uri(URI.create(base + "/api/vehicle/telemetry")).GET().build(),
                  BodyHandlers.ofString())
              .body()
              .contains("terminal-count"));
      assertEquals(
          200,
          client
              .send(
                  HttpRequest.newBuilder()
                      .uri(URI.create(base + "/api/vehicle/main-engine-start"))
                      .header("Content-Type", "application/json")
                      .POST(HttpRequest.BodyPublishers.ofString("{\"command\":\"ignition\"}"))
                      .build(),
                  BodyHandlers.discarding())
              .statusCode());
      assertTrue(
          client
              .send(
                  HttpRequest.newBuilder().uri(URI.create(base + "/api/vehicle/telemetry")).GET().build(),
                  BodyHandlers.ofString())
              .body()
              .contains("first-stage-flight"));
    }
  }

  @Test
  void httpPlaybookBuilder_requestHeaderAndBodyMatch_returnsStubbedResponse() throws Exception {
    OpenArena arena = fluentArena.openArena();
    HttpClient client = HttpClient.newHttpClient();
    String base = "http://127.0.0.1:" + httpPort;
    try (ActiveHttpPlaybook active =
        new HttpPlaybookBuilder(dependencyIdentifier)
            .post("/api/vehicle/ignite")
            .withHeader("Authorization", HttpHeaderPattern.equalTo("Bearer launch-token"))
            .withRequestBody(Map.of("command", "ignition"))
            .willReturn(HttpResponse.okJson(Map.of("accepted", true)))
            .open(arena)) {
      java.net.http.HttpResponse<String> response =
          client.send(
              HttpRequest.newBuilder()
                  .uri(URI.create(base + "/api/vehicle/ignite"))
                  .header("Authorization", "Bearer launch-token")
                  .header("Content-Type", "application/json")
                  .POST(HttpRequest.BodyPublishers.ofString("{\"command\":\"ignition\"}"))
                  .build(),
              BodyHandlers.ofString());
      assertEquals(200, response.statusCode());
      assertTrue(response.body().contains("accepted"));
    }
  }
}
