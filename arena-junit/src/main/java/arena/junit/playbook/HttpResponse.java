package arena.junit.playbook;

import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class HttpResponse {
  private final ObjectNode data;

  public HttpResponse(int status) {
    data = ArenaJson.object();
    data.put("status", status);
  }

  public HttpResponse withStatus(int status) {
    data.put("status", status);
    return this;
  }

  public HttpResponse withJsonBody(Object body) {
    data.set("json_body", ArenaJson.MAPPER.valueToTree(body));
    return this;
  }

  public HttpResponse withHeader(String name, String value) {
    ObjectNode headers =
        data.has("headers")
            ? (ObjectNode) data.get("headers")
            : data.putObject("headers");
    headers.put(name, value);
    return this;
  }

  public HttpResponse withFixedDelayMs(long ms) {
    data.put("fixed_delay_ms", ms);
    return this;
  }

  public HttpResponse withUniformRandomDelayMs(long lower, long upper) {
    ObjectNode dist = ArenaJson.object();
    dist.put("type", "uniform");
    dist.put("lower", lower);
    dist.put("upper", upper);
    data.set("delay_distribution", dist);
    return this;
  }

  ObjectNode forSpec() {
    return data.deepCopy();
  }

  public static HttpResponse ok() {
    return new HttpResponse(200);
  }

  public static HttpResponse okJson(Object body) {
    return new HttpResponse(200).withJsonBody(body);
  }

  public static HttpResponse status(int code) {
    return new HttpResponse(code);
  }

  public static HttpResponse serverError() {
    return new HttpResponse(500);
  }

  public static HttpResponse created() {
    return new HttpResponse(201);
  }

  public static HttpResponse noContent() {
    return new HttpResponse(204);
  }
}
