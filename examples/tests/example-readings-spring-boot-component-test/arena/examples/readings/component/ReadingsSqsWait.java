package arena.examples.readings.component;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.List;
import software.amazon.awssdk.services.sqs.SqsClient;
import software.amazon.awssdk.services.sqs.model.Message;
import software.amazon.awssdk.services.sqs.model.ReceiveMessageRequest;

final class ReadingsSqsWait {

  private ReadingsSqsWait() {}

  static JsonNode waitReadingCreatedDetail(
      ObjectMapper mapper, SqsClient sqs, String queueUrl, int expectedId) throws Exception {
    long deadline = System.currentTimeMillis() + 45_000;
    while (System.currentTimeMillis() < deadline) {
      List<Message> msgs =
          sqs.receiveMessage(
                  ReceiveMessageRequest.builder()
                      .queueUrl(queueUrl)
                      .maxNumberOfMessages(1)
                      .waitTimeSeconds(2)
                      .visibilityTimeout(10)
                      .build())
              .messages();
      for (Message m : msgs) {
        JsonNode body = mapper.readTree(m.body());
        String dtype = body.path("detail-type").asText("");
        if (!"ReadingCreated".equals(dtype)) {
          sqs.deleteMessage(b -> b.queueUrl(queueUrl).receiptHandle(m.receiptHandle()));
          continue;
        }
        JsonNode detail = body.get("detail");
        String detailText = detail.isTextual() ? detail.asText() : detail.toString();
        JsonNode d = detail.isTextual() ? mapper.readTree(detailText) : detail;
        if (d.path("id").asInt(-1) == expectedId) {
          sqs.deleteMessage(b -> b.queueUrl(queueUrl).receiptHandle(m.receiptHandle()));
          return d;
        }
        sqs.deleteMessage(b -> b.queueUrl(queueUrl).receiptHandle(m.receiptHandle()));
      }
    }
    throw new AssertionError("sqs did not receive ReadingCreated for id=" + expectedId);
  }
}
