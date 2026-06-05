package arena.junit.playbook;

import arena.junit.OpenArena;

import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.List;

/**
 * @deprecated {@value ArenaDeprecation#HTTP_PLAYBOOK_BUILDER}
 */
@Deprecated
public final class ActiveHttpPlaybookBuilder {
  private final HttpPlaybookBuilder builder;

  public ActiveHttpPlaybookBuilder(String dependencyIdentifier) {
    this.builder = new HttpPlaybookBuilder(dependencyIdentifier);
  }

  /**
   * @deprecated {@value ArenaDeprecation#HTTP_PLAYBOOK_BUILDER_MAPPING_METHODS}
   */
  @Deprecated
  public ActiveHttpPlaybookBuilder withMapping(
      String method,
      String urlPath,
      int status,
      Object jsonBody,
      Integer priority,
      Integer expectCalled,
      Integer expectCalledAtLeast,
      boolean expectNeverCalled) {
    int expectSet =
        (expectCalled != null ? 1 : 0)
            + (expectCalledAtLeast != null ? 1 : 0)
            + (expectNeverCalled ? 1 : 0);
    if (expectSet > 1) {
      throw new IllegalArgumentException(
          "withMapping accepts at most one of: expectCalled, expectCalledAtLeast, expectNeverCalled");
    }

    HttpMappingBuilder mapping =
        switch (method.toUpperCase()) {
          case "GET" -> builder.get(urlPath);
          case "POST" -> builder.post(urlPath);
          case "PUT" -> builder.put(urlPath);
          case "DELETE" -> builder.delete(urlPath);
          default ->
              throw new IllegalArgumentException("unsupported HTTP method in withMapping: " + method);
        };
    if (priority != null) {
      mapping = mapping.withPriority(priority.intValue());
    }
    HttpSequenceBuilder seq =
        jsonBody != null
            ? mapping.willReturn(status, jsonBody)
            : mapping.willReturn(status);
    if (expectCalled != null) {
      seq = seq.expectCalled(expectCalled.longValue());
    } else if (expectCalledAtLeast != null) {
      seq = seq.expectCalledAtLeast(expectCalledAtLeast.longValue());
    } else if (expectNeverCalled) {
      seq = seq.expectNeverCalled();
    }
    seq.intoPlaybook();
    return this;
  }

  /**
   * @deprecated {@value ArenaDeprecation#HTTP_PLAYBOOK_BUILDER_MAPPING_METHODS}
   */
  @Deprecated
  public ActiveHttpPlaybookBuilder withMapping(String method, String urlPath, int status) {
    return withMapping(method, urlPath, status, null, null, null, null, false);
  }

  /**
   * @deprecated {@value ArenaDeprecation#HTTP_PLAYBOOK_BUILDER_MAPPING_METHODS}
   */
  @Deprecated
  public ActiveHttpPlaybookBuilder withMapping(
      String method, String urlPath, int status, Object jsonBody) {
    return withMapping(method, urlPath, status, jsonBody, null, null, null, false);
  }

  public ActiveHttpPlaybook open(OpenArena arena) {
    return builder.open(arena);
  }

  public List<ObjectNode> mappingsForFfi() {
    return builder.mappingsForFfi();
  }
}
