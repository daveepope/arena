package dev.arena.examples.readings.springboot;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.net.URI;
import java.util.HashMap;
import java.util.Map;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Component;
import software.amazon.awssdk.auth.credentials.AwsBasicCredentials;
import software.amazon.awssdk.auth.credentials.StaticCredentialsProvider;
import software.amazon.awssdk.regions.Region;
import software.amazon.awssdk.services.eventbridge.EventBridgeClient;
import software.amazon.awssdk.services.eventbridge.model.PutEventsRequest;
import software.amazon.awssdk.services.eventbridge.model.PutEventsRequestEntry;

@Component
public class ReadingEventBridge {

  private static final ObjectMapper JSON = new ObjectMapper();

  private final EventBridgeClient client;
  private final String busName;
  private final String source;
  private final String detailType;

  public ReadingEventBridge(
      @Value("${AWS_ENDPOINT_URL:}") String endpoint,
      @Value("${AWS_DEFAULT_REGION}") String region,
      @Value("${AWS_ACCESS_KEY_ID}") String accessKey,
      @Value("${AWS_SECRET_ACCESS_KEY}") String secretKey,
      @Value("${EVENT_BUS_NAME}") String busName,
      @Value("${EVENT_SOURCE}") String source,
      @Value("${READING_CREATED_DETAIL_TYPE:ReadingCreated}") String detailType) {
    this.busName = busName;
    this.source = source;
    this.detailType = detailType;
    var creds = StaticCredentialsProvider.create(AwsBasicCredentials.create(accessKey, secretKey));
    var b = EventBridgeClient.builder().credentialsProvider(creds).region(Region.of(region));
    String ep = endpoint == null ? "" : endpoint.trim();
    if (!ep.isEmpty()) {
      b = b.endpointOverride(URI.create(ep));
    }
    this.client = b.build();
  }

  public void publishReadingCreated(long id, String userName, int value, String comment)
      throws JsonProcessingException {
    Map<String, Object> detail = new HashMap<>();
    detail.put("id", id);
    detail.put("user_name", userName);
    detail.put("value", value);
    detail.put("comment", comment);
    String detailJson = JSON.writeValueAsString(detail);
    PutEventsRequestEntry entry =
        PutEventsRequestEntry.builder()
            .eventBusName(busName)
            .source(source)
            .detailType(detailType)
            .detail(detailJson)
            .build();
    client.putEvents(PutEventsRequest.builder().entries(entry).build());
  }
}
