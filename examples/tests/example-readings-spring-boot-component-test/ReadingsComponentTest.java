package dev.arena.examples.readings.component;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import dev.arena.junit.playbook.ArenaPlaybooks;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.RegisterExtension;
import software.amazon.awssdk.auth.credentials.AwsBasicCredentials;
import software.amazon.awssdk.auth.credentials.StaticCredentialsProvider;
import software.amazon.awssdk.regions.Region;
import software.amazon.awssdk.services.sqs.SqsClient;
import software.amazon.awssdk.services.sqs.model.GetQueueUrlRequest;

public final class ReadingsComponentTest {

  @RegisterExtension
  static final ReadingsArenaFixture readings = new ReadingsArenaFixture();

  @Test
  @ArenaPlaybooks(ReadingsDefaultPlaybooks.class)
  void readingsHappyPathSqsEventAndList() throws Exception {
    Map<String, String> credsMap = readings.awsDummyCredentials();
    var creds =
        StaticCredentialsProvider.create(
            AwsBasicCredentials.create(
                credsMap.get("aws_access_key_id"), credsMap.get("aws_secret_access_key")));
    SqsClient sqs =
        SqsClient.builder()
            .region(Region.of(readings.region()))
            .endpointOverride(URI.create(readings.localstackEndpoint()))
            .credentialsProvider(creds)
            .build();
    String queueUrl =
        sqs.getQueueUrl(GetQueueUrlRequest.builder().queueName(readings.queueName()).build())
            .queueUrl();

    HttpClient http = HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(30)).build();

    String base = "http://127.0.0.1:" + readings.webAppPort();
    String body =
        "{\"user_name\":\"Spring JUnit User\",\"value\":77,\"comment\":\"sqs happy path\"}";
    HttpResponse<String> post =
        http.send(
            HttpRequest.newBuilder()
                .uri(URI.create(base + "/readings"))
                .header("Authorization", "Bearer " + readings.accessToken())
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(body))
                .timeout(Duration.ofSeconds(60))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(200, post.statusCode(), post.body());
    JsonNode created = ReadingsArenaFixture.MAPPER.readTree(post.body());
    assertTrue(created.path("valid").asBoolean(false));
    int rid = created.path("id").asInt();
    assertTrue(rid > 0);

    JsonNode detail =
        ReadingsSqsWait.waitReadingCreatedDetail(
            ReadingsArenaFixture.MAPPER, sqs, queueUrl, rid);
    assertEquals(rid, detail.path("id").asInt());
    assertEquals("Spring JUnit User", detail.path("user_name").asText());
    assertEquals(77, detail.path("value").asInt());
    assertEquals("sqs happy path", detail.path("comment").asText());

    HttpResponse<String> get =
        http.send(
            HttpRequest.newBuilder()
                .uri(URI.create(base + "/readings"))
                .header("Authorization", "Bearer " + readings.accessToken())
                .GET()
                .timeout(Duration.ofSeconds(60))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(200, get.statusCode(), get.body());
    JsonNode rows = ReadingsArenaFixture.MAPPER.readTree(get.body());
    assertTrue(rows.isArray());
    boolean found = false;
    for (JsonNode x : rows) {
      if (x.path("id").asInt() == rid) {
        found = true;
        assertEquals("Spring JUnit User", x.path("user_name").asText());
        assertEquals(77, x.path("value").asInt());
        break;
      }
    }
    assertTrue(found);
  }
}
