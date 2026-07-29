package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.http.ApiClient;
import arena.examples.playbooks.CalibrationApiErrorPathPlaybook;
import arena.examples.playbooks.CalibrationApiFlakyPlaybook;
import arena.examples.playbooks.CalibrationApiHappyPathPlaybook;
import arena.examples.playbooks.EventsPurgePlaybook;
import arena.examples.playbooks.ResetValidationDbPlaybook;
import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.Arena;
import arena.junit.ArenaAfterOpen;
import arena.junit.ArenaBeforeClose;
import arena.junit.ArenaComponent;
import arena.junit.ArenaDependency;
import arena.junit.ArenaLogger;
import arena.junit.ArenaPlaybook;
import arena.junit.Playbook;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.dep.LocalstackDependency;
import arena.junit.dep.LocalstackDependencyBuilder;
import arena.junit.dep.MssqlDependency;
import arena.junit.dep.MssqlDependencyBuilder;
import arena.junit.dep.PostgresDependency;
import arena.junit.dep.PostgresDependencyBuilder;
import arena.junit.dep.smtp.SmtpDependency;
import arena.junit.dep.smtp.SmtpDependencyBuilder;
import arena.junit.dep.temporal.TemporalDependency;
import arena.junit.dep.temporal.TemporalDependencyBuilder;
import arena.junit.exec.ExecutableComponent;
import arena.junit.exec.ExecutableComponentBuilder;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.ffi.ArenaLogLevel;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import arena.junit.playbook.ActiveHttpPlaybook;
import arena.junit.playbook.LocalstackModels;
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
import java.util.Map;
import java.util.UUID;
import javax.net.ssl.SSLContext;
import javax.net.ssl.TrustManagerFactory;
import org.junit.jupiter.api.Test;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import software.amazon.awssdk.auth.credentials.AwsBasicCredentials;
import software.amazon.awssdk.auth.credentials.StaticCredentialsProvider;
import software.amazon.awssdk.regions.Region;
import software.amazon.awssdk.services.sqs.SqsClient;
import software.amazon.awssdk.services.sqs.model.GetQueueUrlRequest;

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
  private static final int CALIBRATION_HOST_PORT = RT.calibrationHostPort;
  private static final int LOCALSTACK_HOST_PORT = RT.localstackHostPort;
  private static final int TEMPORAL_GRPC_PORT = RT.temporalGrpcPort;
  private static final int TEMPORAL_UI_PORT = RT.temporalUiPort;
  private static final int SMTP_HOST_PORT = RT.smtpPort;
  private static final int SMTP_UI_PORT = RT.smtpUiPort;
  private static final String TEMPORAL_TARGET = "127.0.0.1:" + TEMPORAL_GRPC_PORT;
  private static final int OAUTH_PORT = RT.oauthPort;
  private static final String OAUTH_ISSUER = RT.oauthIssuer;
  private static final String POSTGRES_DB_NAME = "readings_db";
  private static final String POSTGRES_DB_USER = "readings_user";
  private static final String POSTGRES_DB_PASS = "readings_password";
  private static final String MSSQL_DB_NAME = "validationDb";
  private static final String MSSQL_DB_USER = "sa";
  private static final String MSSQL_DB_PASS = "yourStrong(!)Password";
  private static final String EVENT_BUS_NAME = "example-api-events";
  private static final String EVENT_SOURCE = "readings.api";
  private static final String QUEUE_NAME = "example-api-events-q";
  private static final String EVENT_RULE_NAME = "example-api-rule";
  private static final String REGION = "us-east-1";
  private static final Map<String, String> AWS_DUMMY =
      Map.of("aws_access_key_id", "test", "aws_secret_access_key", "test");
  private static final String LOCALSTACK_ENDPOINT = "http://127.0.0.1:" + LOCALSTACK_HOST_PORT;

  private static final OauthLoopbackTls.PemPair OAUTH_PEM =
      OauthLoopbackTls.oauthLoopbackTlsPemPair();
  private static final String OAUTH_CA_PATH = writeOauthCaPemFile();

  private static String accessToken;
  private static SqsClient sqsClient;
  private static String sqsQueueUrl;
  private static long readingsDeviceId;

  @ArenaDependency
  static final OauthDependency OAUTH =
      new OauthDependencyBuilder("example-api-oauth")
          .withPort(OAUTH_PORT)
          .withListenIp("0.0.0.0")
          .withServerTlsPem(OAUTH_PEM.certificatePem(), OAUTH_PEM.privateKeyPem())
          .withMetadataBaseUrl(OAUTH_ISSUER)
          .build();

  @ArenaDependency
  static final PostgresDependency POSTGRES =
      new PostgresDependencyBuilder("example-api-postgres")
          .withImage("14.20-trixie")
          .withPort(POSTGRES_PORT)
          .withDatabaseName(POSTGRES_DB_NAME)
          .withDatabaseUsername(POSTGRES_DB_USER)
          .withDatabasePassword(POSTGRES_DB_PASS)
          .withStartupSqlScripts(readSchema("instrument_reading_db_schema.sql"))
          .build();

  @ArenaDependency
  static final MssqlDependency MSSQL =
      new MssqlDependencyBuilder("example-api-mssql")
          .withPort(MSSQL_PORT)
          .withDatabaseName(MSSQL_DB_NAME)
          .withDatabaseUsername(MSSQL_DB_USER)
          .withDatabasePassword(MSSQL_DB_PASS)
          .withStartupSqlScripts(readSchema("validation_db_schema.sql"))
          .build();

  @ArenaDependency
  static final HttpDependency CALIBRATION =
      new HttpDependencyBuilder("example-api-calibration").withPort(CALIBRATION_HOST_PORT).build();

  @ArenaDependency static final LocalstackDependency LOCALSTACK = buildLocalstack();

  @ArenaDependency
  static final TemporalDependency TEMPORAL =
      new TemporalDependencyBuilder("example-api-temporal")
          .withImage("1.8.0")
          .withPort(TEMPORAL_GRPC_PORT)
          .withUiPort(TEMPORAL_UI_PORT)
          .build();

  @ArenaDependency
  static final SmtpDependency SMTP =
      new SmtpDependencyBuilder("example-api-smtp")
          .withPort(SMTP_HOST_PORT)
          .withUiPort(SMTP_UI_PORT)
          .withStarttls()
          .build();

  @ArenaPlaybook
  static final CalibrationApiHappyPathPlaybook CALIBRATION_HAPPY_PATH =
      new CalibrationApiHappyPathPlaybook(CALIBRATION.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final CalibrationApiErrorPathPlaybook CALIBRATION_ERROR_PATH =
      new CalibrationApiErrorPathPlaybook(CALIBRATION.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final CalibrationApiFlakyPlaybook CALIBRATION_FLAKY =
      new CalibrationApiFlakyPlaybook(CALIBRATION.identifier());

  @ArenaPlaybook static final EventsPurgePlaybook EVENTS_PURGE = new EventsPurgePlaybook(LOCALSTACK.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final ResetValidationDbPlaybook RESET_VALIDATION_DB =
      new ResetValidationDbPlaybook(MSSQL.identifier());

  @ArenaComponent static final ExecutableComponent WEB_APP = buildWebApp();

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createReadingPublishesEventAndListsViaHttp() throws Exception {
    int createdId =
        apiClient().createReading("Readings API User", 77, "sqs happy path", readingsDeviceId);
    JsonNode detail = waitReadingCreatedOnQueue(createdId);
    assertEquals(createdId, detail.path("id").asInt());
    assertEquals("Readings API User", detail.path("user_name").asText());
    assertEquals(77, detail.path("value").asInt());
    assertEquals("sqs happy path", detail.path("comment").asText());

    JsonNode found = apiClient().findReadingById(createdId);
    assertEquals("Readings API User", found.path("user_name").asText());
    assertEquals(77, found.path("value").asInt());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createMultipleReadingsAreListed() throws Exception {
    ApiClient client = apiClient();
    int id1 = client.createReading("Bending", 1, "", readingsDeviceId);
    int id2 = client.createReading("joe", 2, "We're going to need a bigger ship", readingsDeviceId);
    assertTrue(client.listReadingIds().contains(id1));
    assertTrue(client.listReadingIds().contains(id2));
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingReturns500WhenCalibrationOutagePlaybookActive() throws Exception {
    HttpResponse<String> response =
        apiClient().postReadingRaw("Outage Test User", 99, null, readingsDeviceId);
    assertEquals(500, response.statusCode(), response.body());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingSucceedsAfterOutagePlaybookScope() throws Exception {
    int recoveredId =
        apiClient().createReading("Recovery Test User", 17, "post-outage", readingsDeviceId);
    JsonNode found = apiClient().findReadingById(recoveredId);
    assertEquals("Recovery Test User", found.path("user_name").asText());
    assertEquals(17, found.path("value").asInt());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createReadingWithValidationDbScopedPlaybook() throws Exception {
    int createdId =
        apiClient().createReading("Validation DB Scoped", 7, "mssql scope", readingsDeviceId);
    assertTrue(apiClient().listReadingIds().contains(createdId));
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingReturns500UnderStackedPlaybooks() throws Exception {
    HttpResponse<String> response =
        apiClient().postReadingRaw("Stack Outage", 1, null, readingsDeviceId);
    assertEquals(500, response.statusCode(), response.body());
  }

  @Test
  @Playbook(CalibrationApiFlakyPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingSucceedsAfterCalibrationFlakySequence() throws Exception {
    ApiClient client = apiClient();
    assertEquals(500, client.postReadingRaw("Flaky 1", 1, null, readingsDeviceId).statusCode());
    assertEquals(500, client.postReadingRaw("Flaky 2", 2, null, readingsDeviceId).statusCode());
    int createdId = client.createReading("Flaky 3", 3, "recovered", readingsDeviceId);
    assertTrue(client.listReadingIds().contains(createdId));
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  void httpPlaybookVerifyAtLeastSucceedsWithTraffic(ActiveHttpPlaybook activeHttpPlaybook)
      throws Exception {
    apiClient().postReadingRaw("Verify At Least", 3, null, readingsDeviceId);
    activeHttpPlaybook.verifyAtLeast("POST", CALIBRATION_VALIDATE_PATH, 1);
  }

  @Test
  void createDeviceRequestTransitionAppliesRequestedState() throws Exception {
    ApiClient client = apiClient();
    long deviceId = client.createDevice("Smell-O-Scope Mk II");
    assertEquals("OFF", client.getDeviceState(deviceId));

    client.setDeviceState(deviceId, "ON");
    assertEquals("ON", client.getDeviceState(deviceId));

    client.setDeviceState(deviceId, "ERROR");
    assertEquals("ERROR", client.getDeviceState(deviceId));

    client.stopDevice(deviceId);
  }

  @Test
  void getDeviceStateUnknownDeviceReturnsNotFound() throws Exception {
    HttpResponse<String> response = apiClient().getDeviceStateRaw(999_999_999L);
    assertEquals(404, response.statusCode());
  }

  @Test
  void createDeviceSendsProvisionedEmailOverStarttls() throws Exception {
    String deviceName = "Mail Probe Device " + UUID.randomUUID().toString().substring(0, 8);
    apiClient().createDevice(deviceName);
    waitDeviceProvisionedEmail(deviceName);
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  void httpPlaybookVerifyCountMismatchRaises(ActiveHttpPlaybook activeHttpPlaybook) {
    assertThrows(
        ArenaBindingError.class,
        () -> activeHttpPlaybook.verify("POST", CALIBRATION_VALIDATE_PATH, 1));
  }

  @ArenaAfterOpen
  static void afterOpen() throws Exception {
    fetchAccessToken();
    readingsDeviceId = apiClient().createDevice("Readings Component Test Device");
    var creds =
        StaticCredentialsProvider.create(
            AwsBasicCredentials.create(
                AWS_DUMMY.get("aws_access_key_id"), AWS_DUMMY.get("aws_secret_access_key")));
    sqsClient =
        SqsClient.builder()
            .region(Region.of(REGION))
            .endpointOverride(URI.create(LOCALSTACK_ENDPOINT))
            .credentialsProvider(creds)
            .build();
    sqsQueueUrl =
        sqsClient
            .getQueueUrl(GetQueueUrlRequest.builder().queueName(QUEUE_NAME).build())
            .queueUrl();
  }

  @ArenaBeforeClose
  static void closeSqsClient() {
    if (sqsClient != null) {
      sqsClient.close();
      sqsClient = null;
    }
    sqsQueueUrl = null;
  }

  private static String writeOauthCaPemFile() {
    try {
      Path ca = Files.createTempFile("example-api-oauth-", ".pem");
      Files.writeString(ca, OAUTH_PEM.certificatePem(), StandardCharsets.UTF_8);
      return ca.toString();
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }

  private static LocalstackDependency buildLocalstack() {
    String lsId = "ls-example-api-" + UUID.randomUUID().toString().substring(0, 8);
    try {
      return new LocalstackDependencyBuilder(lsId)
          .withPort(LOCALSTACK_HOST_PORT)
          .withServices(List.of("sqs", "events"))
          .withQueue(QUEUE_NAME)
          .withEventBus(EVENT_BUS_NAME)
          .withEventRule(
              new LocalstackModels.EventRuleSpec(
                  EVENT_RULE_NAME,
                  MAPPER.writeValueAsString(Map.of("source", List.of(EVENT_SOURCE))),
                  List.of(
                      new LocalstackModels.EventRuleTarget(
                          "target-queue", new LocalstackModels.SqsQueueTarget(QUEUE_NAME))),
                  EVENT_BUS_NAME))
          .build();
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }

  private static ExecutableComponent buildWebApp() {
    String appLauncher;
    try {
      appLauncher = Runfiles.findWebAppLauncher();
    } catch (Exception e) {
      throw new IllegalStateException("failed to locate web app launcher runfile", e);
    }
    assertTrue(!appLauncher.isEmpty(), "web app launcher must be present under Bazel runfiles");
    return new ExecutableComponentBuilder("example-api-web-app")
        .withExecutablePath(appLauncher)
        .withEnvVar("WEB_APP_PORT", String.valueOf(WEB_APP_PORT))
        .withEnvVar(
            "POSTGRES_CONNECTION_STRING",
            "host=localhost port="
                + POSTGRES_PORT
                + " user="
                + POSTGRES_DB_USER
                + " password="
                + POSTGRES_DB_PASS
                + " dbname="
                + POSTGRES_DB_NAME)
        .withEnvVar("CALIBRATION_API_BASE_URL", "http://127.0.0.1:" + CALIBRATION_HOST_PORT)
        .withEnvVar(
            "MSSQL_CONNECTION_STRING",
            "Server=tcp:localhost,"
                + MSSQL_PORT
                + ";Database="
                + MSSQL_DB_NAME
                + ";User Id="
                + MSSQL_DB_USER
                + ";Password="
                + MSSQL_DB_PASS
                + ";TrustServerCertificate=True;")
        .withEnvVar("TEMPORAL_TARGET", TEMPORAL_TARGET)
        .withEnvVar("SMTP_HOST", "127.0.0.1")
        .withEnvVar("SMTP_PORT", String.valueOf(SMTP_HOST_PORT))
        .withEnvVar("OAUTH_ISSUER_URL", OAUTH_ISSUER)
        .withEnvVar("OAUTH_TLS_CA_FILE", OAUTH_CA_PATH)
        .withEnvVar("OAUTH_REQUIRED_ACCESS_TOKEN_SCOPES", "readings")
        .withEnvVar("AWS_ENDPOINT_URL", LOCALSTACK_ENDPOINT)
        .withEnvVar("AWS_DEFAULT_REGION", REGION)
        .withEnvVar("AWS_ACCESS_KEY_ID", AWS_DUMMY.get("aws_access_key_id"))
        .withEnvVar("AWS_SECRET_ACCESS_KEY", AWS_DUMMY.get("aws_secret_access_key"))
        .withEnvVar("EVENT_BUS_NAME", EVENT_BUS_NAME)
        .withEnvVar("EVENT_SOURCE", EVENT_SOURCE)
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

  private static void fetchAccessToken() throws Exception {
    HttpClient client = oauthHttpClient();
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
    String form = "grant_type=client_credentials&client_id=arena-examples&scope=readings";
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

  private static HttpClient oauthHttpClient() throws Exception {
    return HttpClient.newBuilder()
        .sslContext(sslContextFromPemFile(OAUTH_CA_PATH))
        .connectTimeout(Duration.ofSeconds(30))
        .build();
  }

  private static SSLContext sslContextFromPemFile(String path) throws Exception {
    String pem = Files.readString(Path.of(path), StandardCharsets.UTF_8);
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

  private static void waitDeviceProvisionedEmail(String needle) throws Exception {
    String url = "http://127.0.0.1:" + SMTP_UI_PORT + "/api/v1/messages";
    HttpClient client = HttpClient.newHttpClient();
    long deadline = System.currentTimeMillis() + 10_000L;
    while (System.currentTimeMillis() < deadline) {
      HttpResponse<String> response =
          client.send(
              HttpRequest.newBuilder()
                  .uri(URI.create(url))
                  .GET()
                  .timeout(Duration.ofSeconds(5))
                  .build(),
              HttpResponse.BodyHandlers.ofString());
      if (response.statusCode() == 200 && response.body().contains(needle)) {
        return;
      }
      Thread.sleep(100L);
    }
    throw new AssertionError(
        "device provisioned email containing " + needle + " was not captured");
  }

  private static JsonNode waitReadingCreatedOnQueue(int expectedId) throws Exception {
    return SqsWait.waitReadingCreatedDetail(MAPPER, sqsClient, sqsQueueUrl, expectedId);
  }
}
