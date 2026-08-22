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
import java.util.Map;

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
    return readCreatedReadingId(postReadingRaw(userName, value, comment));
  }

  public int createReading(String userName, int value, String comment, long deviceId)
      throws Exception {
    return readCreatedReadingId(postReadingRaw(userName, value, comment, deviceId));
  }

  public HttpResponse<String> postReadingRaw(String userName, int value, String comment)
      throws Exception {
    return post("/readings", readingBody(userName, value, comment, null));
  }

  public HttpResponse<String> postReadingRaw(
      String userName, int value, String comment, long deviceId) throws Exception {
    return post("/readings", readingBody(userName, value, comment, deviceId));
  }

  public long createDevice(String name) throws Exception {
    String body = "{\"name\":" + mapper.writeValueAsString(name) + "}";
    HttpResponse<String> response = post("/devices", body);
    requireOk(response, "POST /devices");
    return mapper.readTree(response.body()).path("id").asLong();
  }

  public String getDeviceState(long deviceId) throws Exception {
    HttpResponse<String> response = getDeviceStateRaw(deviceId);
    requireOk(response, "GET /devices/" + deviceId + "/state");
    return mapper.readTree(response.body()).path("state").asText();
  }

  public HttpResponse<String> getDeviceStateRaw(long deviceId) throws Exception {
    return get("/devices/" + deviceId + "/state");
  }

  public void setDeviceState(long deviceId, String target) throws Exception {
    String body = "{\"target\":" + mapper.writeValueAsString(target) + "}";
    HttpResponse<String> response = post("/devices/" + deviceId + "/state", body);
    requireOk(response, "POST /devices/" + deviceId + "/state");
  }

  public void stopDevice(long deviceId) throws Exception {
    HttpResponse<String> response = delete("/devices/" + deviceId);
    requireOk(response, "DELETE /devices/" + deviceId);
  }

  public List<JsonNode> getReadings() throws Exception {
    HttpResponse<String> response = get("/readings");
    requireOk(response, "GET /readings");
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

  public long createWeatherReport(double precipitation, double humidity, double pressure)
      throws Exception {
    HttpResponse<String> response = postWeatherReportRaw(precipitation, humidity, pressure);
    requireOk(response, "POST /weather");
    return mapper.readTree(response.body()).path("id").asLong();
  }

  public HttpResponse<String> postWeatherReportRaw(
      double precipitation, double humidity, double pressure) throws Exception {
    String body =
        mapper.writeValueAsString(
            Map.of("precipitation", precipitation, "humidity", humidity, "pressure", pressure));
    return post("/weather", body);
  }

  public List<JsonNode> getWeatherReports() throws Exception {
    HttpResponse<String> response = get("/weather");
    requireOk(response, "GET /weather");
    JsonNode rows = mapper.readTree(response.body());
    if (!rows.isArray()) {
      throw new AssertionError("expected weather array: " + response.body());
    }
    List<JsonNode> out = new ArrayList<>();
    for (JsonNode row : rows) {
      out.add(row);
    }
    return out;
  }

  private int readCreatedReadingId(HttpResponse<String> response) throws Exception {
    requireOk(response, "POST /readings");
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

  private String readingBody(String userName, int value, String comment, Long deviceId)
      throws Exception {
    String commentJson = comment == null ? "null" : mapper.writeValueAsString(comment);
    StringBuilder body =
        new StringBuilder("{\"user_name\":")
            .append(mapper.writeValueAsString(userName))
            .append(",\"value\":")
            .append(value)
            .append(",\"comment\":")
            .append(commentJson);
    if (deviceId != null) {
      body.append(",\"device_id\":").append(deviceId);
    }
    return body.append('}').toString();
  }

  private void requireOk(HttpResponse<String> response, String label) {
    if (response.statusCode() != 200) {
      throw new AssertionError(
          label + " failed: " + response.statusCode() + " " + response.body());
    }
  }

  private HttpResponse<String> get(String path) throws Exception {
    return client.send(
        HttpRequest.newBuilder()
            .uri(URI.create(baseUrl + path))
            .header("Authorization", "Bearer " + accessToken)
            .GET()
            .timeout(requestTimeout)
            .build(),
        HttpResponse.BodyHandlers.ofString());
  }

  private HttpResponse<String> post(String path, String body) throws Exception {
    return client.send(
        HttpRequest.newBuilder()
            .uri(URI.create(baseUrl + path))
            .header("Authorization", "Bearer " + accessToken)
            .header("Content-Type", "application/json")
            .POST(HttpRequest.BodyPublishers.ofString(body))
            .timeout(requestTimeout)
            .build(),
        HttpResponse.BodyHandlers.ofString());
  }

  private HttpResponse<String> delete(String path) throws Exception {
    return client.send(
        HttpRequest.newBuilder()
            .uri(URI.create(baseUrl + path))
            .header("Authorization", "Bearer " + accessToken)
            .DELETE()
            .timeout(requestTimeout)
            .build(),
        HttpResponse.BodyHandlers.ofString());
  }
}
