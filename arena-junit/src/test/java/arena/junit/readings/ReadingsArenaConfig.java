package arena.junit.readings;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import arena.junit.oauth.OauthDependencyBuilder;
import java.io.ByteArrayInputStream;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.KeyStore;
import java.security.SecureRandom;
import java.security.cert.Certificate;
import java.security.cert.CertificateFactory;
import java.time.Duration;
import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.Properties;
import java.util.concurrent.TimeUnit;
import javax.net.ssl.SSLContext;
import javax.net.ssl.TrustManagerFactory;
import org.apache.kafka.clients.consumer.ConsumerConfig;
import org.apache.kafka.clients.consumer.ConsumerRecord;
import org.apache.kafka.clients.consumer.ConsumerRecords;
import org.apache.kafka.clients.consumer.KafkaConsumer;
import org.apache.kafka.common.serialization.StringDeserializer;

final class ReadingsArenaConfig {

  static final ObjectMapper MAPPER = new ObjectMapper();

  static final int EXEC_WEB_APP_PORT;
  static final int DOCKER_WEB_HOST_PORT;
  static final int KAFKA_PORT;
  static final String KAFKA_TOPIC;
  static final int CALIBRATION_HOST_PORT;
  static final String CALIBRATION_VALIDATE_PATH;

  static final int POSTGRES_PORT;
  static final String POSTGRES_DB_NAME;
  static final String POSTGRES_DB_USER;
  static final String POSTGRES_DB_PASS;
  static final int MSSQL_PORT;
  static final String MSSQL_DB_NAME;
  static final String MSSQL_DB_USER;
  static final String MSSQL_DB_PASS;

  static final String NETWORK_NAME;
  static final String POSTGRES_CONTAINER_NAME;
  static final String KAFKA_CONTAINER_NAME;
  static final String MSSQL_CONTAINER_NAME;
  static final String CALIBRATION_CONTAINER_NAME;
  static final String DOCKER_IMAGE_TAG;

  static final String CLOSED_ARENA_NAME;
  static final String MATCH_NAME;
  static final String TEMP_DIRECTORY_PREFIX;
  static final String KAFKA_CONSUMER_GROUP_LABEL;

  static final String DEP_NAME_OAUTH;
  static final String DEP_NAME_POSTGRES;
  static final String DEP_NAME_KAFKA;
  static final String DEP_NAME_MSSQL;
  static final String DEP_NAME_CALIBRATION_HTTP;
  static final String COMPONENT_NAME_EXECUTABLE;
  static final String COMPONENT_NAME_CONTAINERIZED;

  static final String PLAYBOOK_CALIBRATION_DEFAULT;
  static final String PLAYBOOK_CALIBRATION_OUTAGE_MANAGED;
  static final String PLAYBOOK_CALIBRATION_OUTAGE_FIXTURE_SCOPE;
  static final String PLAYBOOK_VALIDATION_DB_SCOPED;

  static final String POSTGRES_IMAGE;

  static {
    try {
      JsonNode root = loadRoot();
      JsonNode ports = root.path("ports");
      JsonNode db = root.path("database");
      JsonNode dn = root.path("dependency_names");
      JsonNode cn = root.path("component_names");
      JsonNode ctr = root.path("container_names");
      JsonNode pb = root.path("playbook_names");

      EXEC_WEB_APP_PORT = ports.path("exec_web_app").asInt();
      DOCKER_WEB_HOST_PORT = ports.path("docker_web_host").asInt();
      KAFKA_PORT = ports.path("kafka_host").asInt();
      CALIBRATION_HOST_PORT = ports.path("calibration_host").asInt();
      POSTGRES_PORT = ports.path("postgres_host").asInt();
      MSSQL_PORT = ports.path("mssql_host").asInt();

      KAFKA_TOPIC = root.path("kafka_topic").asText();
      CALIBRATION_VALIDATE_PATH = root.path("calibration_validate_path").asText();

      POSTGRES_DB_NAME = db.path("postgres_name").asText();
      POSTGRES_DB_USER = db.path("postgres_user").asText();
      POSTGRES_DB_PASS = db.path("postgres_password").asText();
      MSSQL_DB_NAME = db.path("mssql_name").asText();
      MSSQL_DB_USER = db.path("mssql_user").asText();
      MSSQL_DB_PASS = db.path("mssql_password").asText();

      NETWORK_NAME = root.path("network_name").asText();
      DOCKER_IMAGE_TAG = root.path("docker_image_tag").asText();
      CLOSED_ARENA_NAME = root.path("closed_arena_name").asText();
      MATCH_NAME = root.path("match_name").asText();
      TEMP_DIRECTORY_PREFIX = root.path("temp_directory_prefix").asText();
      KAFKA_CONSUMER_GROUP_LABEL = root.path("kafka_consumer_group_label").asText();
      POSTGRES_IMAGE = root.path("postgres_image").asText();

      POSTGRES_CONTAINER_NAME = ctr.path("postgres").asText();
      KAFKA_CONTAINER_NAME = ctr.path("kafka").asText();
      MSSQL_CONTAINER_NAME = ctr.path("mssql").asText();
      CALIBRATION_CONTAINER_NAME = ctr.path("calibration").asText();

      DEP_NAME_OAUTH = dn.path("oauth").asText();
      DEP_NAME_POSTGRES = dn.path("postgres").asText();
      DEP_NAME_KAFKA = dn.path("kafka").asText();
      DEP_NAME_MSSQL = dn.path("mssql").asText();
      DEP_NAME_CALIBRATION_HTTP = dn.path("calibration_http").asText();

      COMPONENT_NAME_EXECUTABLE = cn.path("executable").asText();
      COMPONENT_NAME_CONTAINERIZED = cn.path("containerized").asText();

      PLAYBOOK_CALIBRATION_DEFAULT = pb.path("calibration_default").asText();
      PLAYBOOK_CALIBRATION_OUTAGE_MANAGED = pb.path("calibration_outage_managed").asText();
      PLAYBOOK_CALIBRATION_OUTAGE_FIXTURE_SCOPE = pb.path("calibration_outage_fixture_scope").asText();
      PLAYBOOK_VALIDATION_DB_SCOPED = pb.path("validation_db_scoped").asText();
    } catch (Exception e) {
      throw new ExceptionInInitializerError(e);
    }
  }

  private ReadingsArenaConfig() {}

  private static JsonNode loadRoot() throws Exception {
    String p = findConstantsPath();
    if (p == null || p.isEmpty()) {
      throw new IllegalStateException("readings_arena_config.json not found");
    }
    return MAPPER.readTree(Files.readString(Path.of(p)));
  }

  private static String findConstantsPath() throws Exception {
    String rf = System.getenv("RUNFILES_DIR");
    if (rf != null) {
      for (String rel :
          List.of(
              "arena/examples/resources/readings_arena_config.json",
              "examples/resources/readings_arena_config.json")) {
        for (String base : List.of("_main", "arena", "")) {
          Path p = base.isEmpty() ? Path.of(rf, rel) : Path.of(rf, base, rel);
          if (Files.isRegularFile(p)) {
            return p.toAbsolutePath().toString();
          }
        }
      }
    }
    return "";
  }

  static String baseUrlExec() {
    return "http://127.0.0.1:" + EXEC_WEB_APP_PORT;
  }

  static String baseUrlDocker() {
    return "http://127.0.0.1:" + DOCKER_WEB_HOST_PORT;
  }

  static String findRunfile(String... candidates) throws Exception {
    String rf = System.getenv("RUNFILES_DIR");
    if (rf != null) {
      for (String rel : candidates) {
        for (String base : List.of("_main", "arena", "")) {
          Path p = base.isEmpty() ? Path.of(rf, rel) : Path.of(rf, base, rel);
          if (Files.isRegularFile(p)) {
            return p.toAbsolutePath().toString();
          }
        }
      }
    }
    return "";
  }

  static String findSchema(String filename) throws Exception {
    return findRunfile(
        "arena/examples/resources/" + filename,
        "_main/examples/resources/" + filename,
        "examples/resources/" + filename);
  }

  static String findAxumBinary() throws Exception {
    return findRunfile(
        "arena/examples/example-readings-axum-web-app",
        "_main/examples/example-readings-axum-web-app",
        "examples/example-readings-axum-web-app");
  }

  static SSLContext sslContextFromPem(String pem) throws Exception {
    CertificateFactory cf = CertificateFactory.getInstance("X.509");
    Collection<? extends Certificate> certs =
        cf.generateCertificates(new ByteArrayInputStream(pem.getBytes(StandardCharsets.UTF_8)));
    KeyStore ks = KeyStore.getInstance(KeyStore.getDefaultType());
    ks.load(null);
    int i = 0;
    for (Certificate c : certs) {
      ks.setCertificateEntry("ca" + i++, c);
    }
    TrustManagerFactory tmf =
        TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm());
    tmf.init(ks);
    SSLContext ctx = SSLContext.getInstance("TLS");
    ctx.init(null, tmf.getTrustManagers(), new SecureRandom());
    return ctx;
  }

  static HttpClient oauthClient(String oauthCaPem) throws Exception {
    return HttpClient.newBuilder()
        .sslContext(sslContextFromPem(oauthCaPem))
        .connectTimeout(Duration.ofSeconds(30))
        .build();
  }

  static String fetchAccessToken(String oauthCaPem, String cachedToken) throws Exception {
    if (cachedToken != null && !cachedToken.isEmpty()) {
      return cachedToken;
    }
    HttpClient c = oauthClient(oauthCaPem);
    String issuer = OauthDependencyBuilder.OAUTH_ISSUER;
    HttpResponse<String> disc =
        c.send(
            HttpRequest.newBuilder()
                .uri(URI.create(issuer + "/.well-known/oauth-authorization-server"))
                .GET()
                .timeout(Duration.ofSeconds(30))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(200, disc.statusCode(), disc.body());
    String tokenUrl = MAPPER.readTree(disc.body()).get("token_endpoint").asText();
    String form = "grant_type=client_credentials&client_id=arena-examples";
    HttpResponse<String> tok =
        c.send(
            HttpRequest.newBuilder()
                .uri(URI.create(tokenUrl))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .POST(HttpRequest.BodyPublishers.ofString(form))
                .timeout(Duration.ofSeconds(30))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(200, tok.statusCode(), tok.body());
    return MAPPER.readTree(tok.body()).get("access_token").asText();
  }

  static String mssqlConnectionLocal() {
    return "Server=tcp:localhost,"
        + MSSQL_PORT
        + ";Database="
        + MSSQL_DB_NAME
        + ";User Id="
        + MSSQL_DB_USER
        + ";Password="
        + MSSQL_DB_PASS
        + ";TrustServerCertificate=True;";
  }

  static String mssqlConnectionDocker() {
    return "Server=tcp:"
        + MSSQL_CONTAINER_NAME
        + ",1433;Database="
        + MSSQL_DB_NAME
        + ";User Id="
        + MSSQL_DB_USER
        + ";Password="
        + MSSQL_DB_PASS
        + ";TrustServerCertificate=True;";
  }

  static String postgresConnectionLocal() {
    return "host=localhost port="
        + POSTGRES_PORT
        + " user="
        + POSTGRES_DB_USER
        + " password="
        + POSTGRES_DB_PASS
        + " dbname="
        + POSTGRES_DB_NAME;
  }

  static String postgresConnectionDocker() {
    return "host="
        + POSTGRES_CONTAINER_NAME
        + " port=5432 user="
        + POSTGRES_DB_USER
        + " password="
        + POSTGRES_DB_PASS
        + " dbname="
        + POSTGRES_DB_NAME;
  }

  static String kafkaBootstrapDocker(String kafkaContainerName, int internalPort) {
    return kafkaContainerName + ":" + internalPort;
  }

  static String runtimeContainerfile() {
    return "FROM debian:trixie-slim\n"
        + "RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*\n"
        + "COPY example-readings-axum-web-app /usr/local/bin/example-readings-axum-web-app\n"
        + "RUN chmod +x /usr/local/bin/example-readings-axum-web-app\n"
        + "EXPOSE 3000\n"
        + "ENTRYPOINT [\"/usr/local/bin/example-readings-axum-web-app\"]\n";
  }

  static Path prepareDockerImageContext(String binaryPath) throws Exception {
    if (binaryPath == null || binaryPath.isEmpty() || !Files.isRegularFile(Path.of(binaryPath))) {
      return null;
    }
    Path ctx = Files.createTempDirectory(TEMP_DIRECTORY_PREFIX);
    Path dst = ctx.resolve("example-readings-axum-web-app");
    Files.copy(Path.of(binaryPath), dst);
    Files.writeString(ctx.resolve("Containerfile"), runtimeContainerfile(), StandardCharsets.UTF_8);
    return ctx;
  }

  static HttpClient readingsClient() {
    return HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(10)).build();
  }

  static HttpRequest.Builder auth(String token) {
    return HttpRequest.newBuilder().header("Authorization", "Bearer " + token);
  }

  static JsonNode postJson(HttpClient c, String url, String token, String json) throws Exception {
    HttpResponse<String> r =
        c.send(
            auth(token)
                .uri(URI.create(url))
                .header("Content-Type", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(json))
                .timeout(Duration.ofSeconds(10))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(200, r.statusCode(), r.body());
    return MAPPER.readTree(r.body());
  }

  static List<JsonNode> getReadings(HttpClient c, String base, String token) throws Exception {
    HttpResponse<String> r =
        c.send(
            auth(token)
                .uri(URI.create(base + "/readings"))
                .GET()
                .timeout(Duration.ofSeconds(10))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(200, r.statusCode(), r.body());
    JsonNode tree = MAPPER.readTree(r.body());
    List<JsonNode> out = new ArrayList<>();
    tree.forEach(out::add);
    return out;
  }

  static int createReading(
      HttpClient c, String base, String token, String user, int value, String comment)
      throws Exception {
    String cmt = comment == null ? "null" : MAPPER.writeValueAsString(comment);
    String body =
        "{\"user_name\":"
            + MAPPER.writeValueAsString(user)
            + ",\"value\":"
            + value
            + ",\"comment\":"
            + cmt
            + "}";
    JsonNode j = postJson(c, base + "/readings", token, body);
    return j.get("id").asInt();
  }

  static JsonNode consumeReadingCreated(String bootstrap, int expectedId, String groupPrefix)
      throws Exception {
    Properties p = new Properties();
    p.put(ConsumerConfig.BOOTSTRAP_SERVERS_CONFIG, bootstrap);
    p.put(
        ConsumerConfig.GROUP_ID_CONFIG,
        KAFKA_CONSUMER_GROUP_LABEL + "-" + groupPrefix + "-" + ProcessHandle.current().pid());
    p.put(ConsumerConfig.KEY_DESERIALIZER_CLASS_CONFIG, StringDeserializer.class.getName());
    p.put(ConsumerConfig.VALUE_DESERIALIZER_CLASS_CONFIG, StringDeserializer.class.getName());
    p.put(ConsumerConfig.AUTO_OFFSET_RESET_CONFIG, "earliest");
    try (KafkaConsumer<String, String> consumer = new KafkaConsumer<>(p)) {
      consumer.subscribe(List.of(KAFKA_TOPIC));
      long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(15);
      while (System.nanoTime() < deadline) {
        ConsumerRecords<String, String> recs = consumer.poll(Duration.ofMillis(200));
        for (ConsumerRecord<String, String> r : recs) {
          if (r.value() == null) {
            continue;
          }
          JsonNode ev = MAPPER.readTree(r.value());
          if (ev.path("id").asInt(-1) == expectedId) {
            return ev;
          }
        }
      }
    }
    throw new AssertionError("did not receive ReadingCreatedEvent before timeout");
  }
}
