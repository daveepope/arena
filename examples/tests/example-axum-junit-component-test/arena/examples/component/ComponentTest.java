package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.http.ApiClient;
import arena.examples.playbooks.CalibrationApiErrorPathPlaybook;
import arena.examples.playbooks.CalibrationApiFlakyPlaybook;
import arena.examples.playbooks.CalibrationApiHappyPathPlaybook;
import arena.examples.playbooks.ResetValidationDbPlaybook;
import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.Arena;
import arena.junit.ArenaAfterOpen;
import arena.junit.ArenaComponent;
import arena.junit.ArenaDependency;
import arena.junit.ArenaLogger;
import arena.junit.ArenaPlaybook;
import arena.junit.Playbook;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.dep.KafkaDependency;
import arena.junit.dep.KafkaDependencyBuilder;
import arena.junit.dep.KafkaFlavor;
import arena.junit.dep.MssqlDependency;
import arena.junit.dep.MssqlDependencyBuilder;
import arena.junit.dep.PostgresDependency;
import arena.junit.dep.PostgresDependencyBuilder;
import arena.junit.exec.ExecutableComponent;
import arena.junit.exec.ExecutableComponentBuilder;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.ffi.ArenaLogLevel;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import arena.junit.playbook.ActiveHttpPlaybook;
import arena.junit.readiness.HttpReadinessCheck;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.UncheckedIOException;
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
import java.util.Collection;
import java.util.List;
import javax.net.ssl.SSLContext;
import javax.net.ssl.TrustManagerFactory;
import org.junit.jupiter.api.Test;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

@Arena
public final class ComponentTest {

  static final ObjectMapper MAPPER = new ObjectMapper();
  static final String CALIBRATION_VALIDATE_PATH = "/api/v1/validate";

  @ArenaLogger(level = ArenaLogLevel.DEBUG)
  static final Logger LOG = LoggerFactory.getLogger(ComponentTest.class);

  private static final EphemeralTestRuntime RT = EphemeralTestRuntime.get();
  private static final int WEB_APP_PORT = RT.execWebAppPort;
  private static final int POSTGRES_PORT = RT.postgresPort;
  private static final int MSSQL_PORT = RT.mssqlPort;
  private static final int KAFKA_PORT = RT.kafkaPort;
  private static final int CALIBRATION_HOST_PORT = RT.calibrationHostPort;
  private static final int OAUTH_PORT = RT.oauthPort;
  private static final String OAUTH_ISSUER = RT.oauthIssuer;
  private static final String POSTGRES_DB_NAME = "readings_db";
  private static final String POSTGRES_DB_USER = "readings_user";
  private static final String POSTGRES_DB_PASS = "readings_password";
  private static final String MSSQL_DB_NAME = "validationDb";
  private static final String MSSQL_DB_USER = "sa";
  private static final String MSSQL_DB_PASS = "yourStrong(!)Password";
  private static final String KAFKA_TOPIC = "readings";

  private static final OauthLoopbackTls.PemPair OAUTH_PEM =
      OauthLoopbackTls.oauthLoopbackTlsPemPair();

  private static String accessToken;

  @ArenaDependency(logs = true)
  static final OauthDependency OAUTH =
      new OauthDependencyBuilder("example-api-oauth")
          .withPort(OAUTH_PORT)
          .withListenIp("0.0.0.0")
          .withServerTlsPem(OAUTH_PEM.certificatePem(), OAUTH_PEM.privateKeyPem())
          .withMetadataBaseUrl(OAUTH_ISSUER)
          .build();

  @ArenaDependency(logs = true)
  static final PostgresDependency POSTGRES =
      new PostgresDependencyBuilder("example-api-postgres")
          .withImage("14.20-trixie")
          .withPort(POSTGRES_PORT)
          .withDatabaseName(POSTGRES_DB_NAME)
          .withDatabaseUsername(POSTGRES_DB_USER)
          .withDatabasePassword(POSTGRES_DB_PASS)
          .withStartupSqlScripts(readSchema("instrument_reading_db_schema.sql"))
          .build();

  @ArenaDependency(logs = true)
  static final KafkaDependency KAFKA =
      new KafkaDependencyBuilder("example-api-kafka")
          .withFlavor(KafkaFlavor.APACHE_NATIVE)
          .withPort(KAFKA_PORT)
          .withTopic(KAFKA_TOPIC)
          .build();

  @ArenaDependency(logs = true)
  static final MssqlDependency MSSQL =
      new MssqlDependencyBuilder("example-api-mssql")
          .withPort(MSSQL_PORT)
          .withDatabaseName(MSSQL_DB_NAME)
          .withDatabaseUsername(MSSQL_DB_USER)
          .withDatabasePassword(MSSQL_DB_PASS)
          .withStartupSqlScripts(readSchema("validation_db_schema.sql"))
          .build();

  @ArenaDependency(logs = true)
  static final HttpDependency CALIBRATION =
      new HttpDependencyBuilder("example-api-calibration").withPort(CALIBRATION_HOST_PORT).build();

  @ArenaPlaybook
  static final CalibrationApiHappyPathPlaybook CALIBRATION_HAPPY_PATH =
      new CalibrationApiHappyPathPlaybook(CALIBRATION.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final CalibrationApiErrorPathPlaybook CALIBRATION_ERROR_PATH =
      new CalibrationApiErrorPathPlaybook(CALIBRATION.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final CalibrationApiFlakyPlaybook CALIBRATION_FLAKY =
      new CalibrationApiFlakyPlaybook(CALIBRATION.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final ResetValidationDbPlaybook RESET_VALIDATION_DB =
      new ResetValidationDbPlaybook(MSSQL.identifier());

  @ArenaComponent(logs = true)
  static final ExecutableComponent WEB_APP = buildWebApp();

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createReadingPublishesEventAndListsViaHttp() throws Exception {
    JsonNode detail =
        waitReadingCreatedOnKafka(
            () -> apiClient().createReading("Readings API User", 77, "kafka happy path"));
    int createdId = detail.path("id").asInt();
    assertEquals(createdId, detail.path("id").asInt());
    assertEquals("Readings API User", detail.path("user_name").asText());
    assertEquals(77, detail.path("value").asInt());
    assertEquals("kafka happy path", detail.path("comment").asText());

    JsonNode found = apiClient().findReadingById(createdId);
    assertEquals("Readings API User", found.path("user_name").asText());
    assertEquals(77, found.path("value").asInt());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createMultipleReadingsAreListed() throws Exception {
    ApiClient client = apiClient();
    int id1 = client.createReading("Bending", 1, "");
    int id2 = client.createReading("joe", 2, "We're going to need a bigger ship");
    assertTrue(client.listReadingIds().contains(id1));
    assertTrue(client.listReadingIds().contains(id2));
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingReturns500WhenCalibrationOutagePlaybookActive() throws Exception {
    HttpResponse<String> response = apiClient().postReadingRaw("Outage Test User", 99, null);
    assertEquals(500, response.statusCode(), response.body());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingSucceedsAfterOutagePlaybookScope() throws Exception {
    int recoveredId = apiClient().createReading("Recovery Test User", 17, "post-outage");
    JsonNode found = apiClient().findReadingById(recoveredId);
    assertEquals("Recovery Test User", found.path("user_name").asText());
    assertEquals(17, found.path("value").asInt());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createReadingWithValidationDbScopedPlaybook() throws Exception {
    int createdId = apiClient().createReading("Validation DB Scoped", 7, "mssql scope");
    assertTrue(apiClient().listReadingIds().contains(createdId));
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingReturns500UnderStackedPlaybooks() throws Exception {
    HttpResponse<String> response = apiClient().postReadingRaw("Stack Outage", 1, null);
    assertEquals(500, response.statusCode(), response.body());
  }

  @Test
  @Playbook(CalibrationApiFlakyPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingSucceedsAfterCalibrationFlakySequence() throws Exception {
    ApiClient client = apiClient();
    assertEquals(500, client.postReadingRaw("Flaky 1", 1, null).statusCode());
    assertEquals(500, client.postReadingRaw("Flaky 2", 2, null).statusCode());
    int createdId = client.createReading("Flaky 3", 3, "recovered");
    assertTrue(client.listReadingIds().contains(createdId));
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  void httpPlaybookVerifyAtLeastSucceedsWithTraffic(ActiveHttpPlaybook activeHttpPlaybook)
      throws Exception {
    apiClient().postReadingRaw("Verify At Least", 3, null);
    activeHttpPlaybook.verifyAtLeast("POST", CALIBRATION_VALIDATE_PATH, 1);
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  void httpPlaybookVerifyCountMismatchRaises(ActiveHttpPlaybook activeHttpPlaybook) {
    assertThrows(
        ArenaBindingError.class,
        () -> activeHttpPlaybook.verify("POST", CALIBRATION_VALIDATE_PATH, 1));
  }

  @ArenaAfterOpen
  static void fetchAccessToken() throws Exception {
    HttpClient client =
        HttpClient.newBuilder()
            .sslContext(sslContextFromPem(OAUTH_PEM.certificatePem()))
            .connectTimeout(Duration.ofSeconds(30))
            .build();
    HttpResponse<String> disc =
        client.send(
            HttpRequest.newBuilder()
                .uri(URI.create(OAUTH_ISSUER + "/.well-known/oauth-authorization-server"))
                .GET()
                .timeout(Duration.ofSeconds(30))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(200, disc.statusCode(), disc.body());
    String tokenUrl = MAPPER.readTree(disc.body()).get("token_endpoint").asText();
    String form =
        "grant_type=client_credentials&client_id=arena-examples&scope=openid%20profile%20readings";
    HttpResponse<String> tok =
        client.send(
            HttpRequest.newBuilder()
                .uri(URI.create(tokenUrl))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .POST(HttpRequest.BodyPublishers.ofString(form))
                .timeout(Duration.ofSeconds(30))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(200, tok.statusCode(), tok.body());
    accessToken = MAPPER.readTree(tok.body()).get("access_token").asText();
  }

  private static ExecutableComponent buildWebApp() {
    String bin;
    try {
      bin = Runfiles.findAxumBinary();
    } catch (Exception e) {
      throw new IllegalStateException("failed to locate example-readings-axum-web-app runfile", e);
    }
    assertTrue(!bin.isEmpty(), "example-readings-axum-web-app must be present under Bazel runfiles");
    return new ExecutableComponentBuilder("example-api-web-app")
        .withExecutablePath(bin)
        .withEnvVar("RUST_LOG", "info")
        .withEnvVar("OAUTH_TLS_CA_PEM", OAUTH_PEM.certificatePem())
        .withRuntimeArg("web_app_port", String.valueOf(WEB_APP_PORT))
        .withRuntimeArg(
            "postgres_connection_string",
            "host=localhost port="
                + POSTGRES_PORT
                + " user="
                + POSTGRES_DB_USER
                + " password="
                + POSTGRES_DB_PASS
                + " dbname="
                + POSTGRES_DB_NAME)
        .withRuntimeArg("kafka_bootstrap", "localhost:" + KAFKA_PORT)
        .withRuntimeArg("calibration_url", "http://127.0.0.1:" + CALIBRATION_HOST_PORT)
        .withRuntimeArg(
            "mssql_connection_string",
            "Server=tcp:localhost,"
                + MSSQL_PORT
                + ";Database="
                + MSSQL_DB_NAME
                + ";User Id="
                + MSSQL_DB_USER
                + ";Password="
                + MSSQL_DB_PASS
                + ";TrustServerCertificate=True;encrypt=DANGER_PLAINTEXT;")
        .withRuntimeArg("oauth_issuer_url", OAUTH_ISSUER)
        .withReadinessCheck(
            HttpReadinessCheck.create(),
            "http://127.0.0.1:" + WEB_APP_PORT + "/health",
            30_000L)
        .build();
  }

  private static List<String> readSchema(String filename) {
    String path;
    try {
      path = Runfiles.findSchema(filename);
    } catch (Exception e) {
      throw new IllegalStateException("failed to locate schema runfile " + filename, e);
    }
    assertTrue(!path.isEmpty(), filename);
    try {
      return List.of(Files.readString(Path.of(path), StandardCharsets.UTF_8));
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }

  private static SSLContext sslContextFromPem(String pem) throws Exception {
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

  private static ApiClient apiClient() {
    return new ApiClient("http://127.0.0.1:" + WEB_APP_PORT, accessToken, MAPPER);
  }

  private static JsonNode waitReadingCreatedOnKafka(KafkaWait.ReadingCreateAction create)
      throws Exception {
    return KafkaWait.waitReadingCreatedDetail(MAPPER, "localhost:" + KAFKA_PORT, KAFKA_TOPIC, create);
  }
}
