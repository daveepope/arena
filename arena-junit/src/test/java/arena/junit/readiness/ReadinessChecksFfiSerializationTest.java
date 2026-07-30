package arena.junit.readiness;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.fasterxml.jackson.databind.node.ArrayNode;

import java.util.List;

import org.junit.jupiter.api.Test;

final class ReadinessChecksFfiSerializationTest {

  @Test
  void httpCheck_serializesKindHttp() {
    ArrayNode out =
        ReadinessChecksFfi.forExecutable(
            List.of(
                new ReadinessChecksFfi.ReadinessEntry(
                    HttpReadinessCheck.create(), "http://127.0.0.1:8080/health")));
    assertEquals(1, out.size());
    assertEquals("http", out.get(0).path("kind").asText());
  }

  @Test
  void tcpCheck_serializesKindTcp() {
    ArrayNode out =
        ReadinessChecksFfi.forExecutable(
            List.of(
                new ReadinessChecksFfi.ReadinessEntry(
                    TcpReadinessCheck.create(), "127.0.0.1:2525", 5_000L)));
    assertEquals(1, out.size());
    assertEquals("tcp", out.get(0).path("kind").asText());
    assertEquals(5_000, out.get(0).path("timeout_ms").asInt());
  }
}
