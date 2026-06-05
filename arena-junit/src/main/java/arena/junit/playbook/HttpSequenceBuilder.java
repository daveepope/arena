package arena.junit.playbook;

import arena.junit.OpenArena;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class HttpSequenceBuilder {
  private final HttpPlaybookBuilder playbook;
  private final ObjectNode mappingSpec;
  private final List<HttpResponse> responses;
  private ObjectNode expect;

  HttpSequenceBuilder(
      HttpPlaybookBuilder playbook, ObjectNode mappingSpec, List<HttpResponse> responses) {
    this.playbook = playbook;
    this.mappingSpec = mappingSpec;
    this.responses = new ArrayList<>(responses);
  }

  public HttpSequenceBuilder thenReturn(HttpResponse response) {
    responses.add(response);
    return this;
  }

  public HttpSequenceBuilder thenReturn(int status) {
    return thenReturn(HttpResponse.status(status));
  }

  public HttpSequenceBuilder thenReturn(int status, Object jsonBody) {
    return thenReturn(HttpResponse.status(status).withJsonBody(jsonBody));
  }

  public HttpSequenceBuilder expectCalled(long count) {
    expect = ArenaJson.object();
    expect.put("kind", "exactly");
    expect.put("count", count);
    return this;
  }

  public HttpSequenceBuilder expectCalledAtLeast(long count) {
    expect = ArenaJson.object();
    expect.put("kind", "at_least");
    expect.put("count", count);
    return this;
  }

  public HttpSequenceBuilder expectNeverCalled() {
    expect = ArenaJson.object();
    expect.put("kind", "never");
    return this;
  }

  private ObjectNode finalizeSpec() {
    ObjectNode out = mappingSpec.deepCopy();
    if (responses.size() == 1) {
      out.set("response", responses.get(0).forSpec());
    } else {
      ArrayNode arr = ArenaJson.array();
      for (HttpResponse response : responses) {
        arr.add(response.forSpec());
      }
      out.set("responses", arr);
    }
    if (expect != null) {
      out.set("expect", expect.deepCopy());
    }
    return out;
  }

  public HttpPlaybookBuilder intoPlaybook() {
    playbook.appendMapping(finalizeSpec());
    return playbook;
  }

  public HttpMappingBuilder get(String urlPath) {
    return intoPlaybook().get(urlPath);
  }

  public HttpMappingBuilder post(String urlPath) {
    return intoPlaybook().post(urlPath);
  }

  public HttpMappingBuilder put(String urlPath) {
    return intoPlaybook().put(urlPath);
  }

  public HttpMappingBuilder delete(String urlPath) {
    return intoPlaybook().delete(urlPath);
  }

  public ActiveHttpPlaybook open(OpenArena arena) {
    return intoPlaybook().open(arena);
  }
}
