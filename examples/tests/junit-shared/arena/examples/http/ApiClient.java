package arena.examples.http;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;

public final class ApiClient {

  private static final HttpClient SHARED_CLIENT =
      HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(30)).build();

  private final HttpClient client;
  private final String baseUrl;
  private final String accessToken;
  private final ObjectMapper mapper;
  private final Duration requestTimeout;

  public ApiClient(String baseUrl, String accessToken, ObjectMapper mapper) {
    this(SHARED_CLIENT, baseUrl, accessToken, mapper, Duration.ofSeconds(10));
  }

  public ApiClient(
      HttpClient client,
      String baseUrl,
      String accessToken,
      ObjectMapper mapper,
      Duration requestTimeout) {
    this.client = client;
    this.baseUrl = baseUrl.endsWith("/") ? baseUrl.substring(0, baseUrl.length() - 1) : baseUrl;
    this.accessToken = accessToken;
    this.mapper = mapper;
    this.requestTimeout = requestTimeout;
  }

  public int createReading(String userName, int value, String comment) throws Exception {
    HttpResponse<String> response = postReadingRaw(userName, value, comment);
    if (response.statusCode() != 200) {
      throw new AssertionError("POST /readings failed: " + response.statusCode() + " " + response.body());
    }
    JsonNode created = mapper.readTree(response.body());
    if (!created.path("valid").asBoolean(false)) {
      throw new AssertionError("expected valid=true in response: " + response.body());
    }
    int id = created.path("id").asInt();
    if (id <= 0) {
      throw new AssertionError("expected positive id in response: " + response.body());
    }
    return id;
  }

  public HttpResponse<String> postReadingRaw(String userName, int value, String comment)
      throws Exception {
    String commentJson = comment == null ? "null" : mapper.writeValueAsString(comment);
    String body =
        "{\"user_name\":"
            + mapper.writeValueAsString(userName)
            + ",\"value\":"
            + value
            + ",\"comment\":"
            + commentJson
            + "}";
    return client.send(
        HttpRequest.newBuilder()
            .uri(URI.create(baseUrl + "/readings"))
            .header("Authorization", "Bearer " + accessToken)
            .header("Content-Type", "application/json")
            .POST(HttpRequest.BodyPublishers.ofString(body))
            .timeout(requestTimeout)
            .build(),
        HttpResponse.BodyHandlers.ofString());
  }

  public List<JsonNode> getReadings() throws Exception {
    HttpResponse<String> response =
        client.send(
            HttpRequest.newBuilder()
                .uri(URI.create(baseUrl + "/readings"))
                .header("Authorization", "Bearer " + accessToken)
                .GET()
                .timeout(requestTimeout)
                .build(),
            HttpResponse.BodyHandlers.ofString());
    if (response.statusCode() != 200) {
      throw new AssertionError("GET /readings failed: " + response.statusCode() + " " + response.body());
    }
    JsonNode rows = mapper.readTree(response.body());
    if (!rows.isArray()) {
      throw new AssertionError("expected readings array: " + response.body());
    }
    List<JsonNode> out = new ArrayList<>();
    for (JsonNode row : rows) {
      out.add(row);
    }
    return out;
  }

  public List<Integer> listReadingIds() throws Exception {
    List<Integer> ids = new ArrayList<>();
    for (JsonNode row : getReadings()) {
      ids.add(row.path("id").asInt());
    }
    return ids;
  }

  public JsonNode findReadingById(int id) throws Exception {
    for (JsonNode row : getReadings()) {
      if (row.path("id").asInt() == id) {
        return row;
      }
    }
    throw new AssertionError("reading id not listed: " + id);
  }
}
