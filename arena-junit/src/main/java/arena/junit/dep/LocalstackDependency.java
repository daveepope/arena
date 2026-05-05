package arena.junit.dep;
import arena.junit.match.ArenaMatchPiece;
import arena.junit.playbook.LocalstackModels;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

public final class LocalstackDependency implements ArenaMatchPiece {
  private final ObjectNode config;

  LocalstackDependency(ObjectNode config) {
    this.config = config;
  }

  @Override
  public ObjectNode forFfi() {
    return config;
  }

  public String identifier() {
    return config.get("identifier").asText();
  }

  public int port() {
    if (config.has("port")) {
      return config.get("port").asInt(LocalstackModels.LOCALSTACK_INTERNAL_DOCKER_PORT);
    }
    return LocalstackModels.LOCALSTACK_INTERNAL_DOCKER_PORT;
  }

  public String endpointUrl(String host) {
    return "http://" + host + ":" + port();
  }

  public String internalEndpointUrl(String containerName) {
    String name =
        containerName != null && !containerName.isEmpty()
            ? containerName
            : config.path("container_name").asText(identifier());
    return "http://" + name + ":" + LocalstackModels.LOCALSTACK_INTERNAL_DOCKER_PORT;
  }

  public String queueUrl(String queueName, String host) {
    return endpointUrl(host)
        + "/"
        + LocalstackModels.LOCALSTACK_DEFAULT_ACCOUNT_ID
        + "/"
        + queueName;
  }

  public String queueArn(String queueName, String region) {
    return "arn:aws:sqs:"
        + region
        + ":"
        + LocalstackModels.LOCALSTACK_DEFAULT_ACCOUNT_ID
        + ":"
        + queueName;
  }

  public String lambdaArn(String functionName, String region) {
    return "arn:aws:lambda:"
        + region
        + ":"
        + LocalstackModels.LOCALSTACK_DEFAULT_ACCOUNT_ID
        + ":function:"
        + functionName;
  }
}
