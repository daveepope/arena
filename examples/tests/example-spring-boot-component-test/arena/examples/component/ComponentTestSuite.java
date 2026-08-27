package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.http.ApiClient;
import arena.examples.playbooks.CalibrationApiErrorPathPlaybook;
import arena.examples.playbooks.CalibrationApiFlakyPlaybook;
import arena.examples.playbooks.CalibrationApiHappyPathPlaybook;
import arena.examples.playbooks.EventsPurgePlaybook;
import arena.examples.playbooks.ResetReadingsDbPlaybook;
import arena.examples.playbooks.ResetValidationDbPlaybook;
import arena.examples.playbooks.ResetWeatherDbPlaybook;
import arena.examples.playbooks.SeedValidationReadingPlaybook;
import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.Arena;
import arena.junit.ArenaAfterOpen;
import arena.junit.ArenaBeforeClose;
import arena.junit.ArenaComponent;
import arena.junit.ArenaDependency;
import arena.junit.ArenaLogger;
import arena.junit.ArenaPlaybook;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.dep.LocalstackDependency;
import arena.junit.dep.LocalstackDependencyBuilder;
import arena.junit.dep.MssqlDependency;
import arena.junit.dep.MssqlDependencyBuilder;
import arena.junit.dep.PostgresDependency;
import arena.junit.dep.PostgresDependencyBuilder;
import arena.junit.dep.oracledb.OracleDependency;
import arena.junit.dep.oracledb.OracleDependencyBuilder;
import arena.junit.dep.smtp.SmtpDependency;
import arena.junit.dep.smtp.SmtpDependencyBuilder;
import arena.junit.dep.temporal.TemporalDependency;
import arena.junit.dep.temporal.TemporalDependencyBuilder;
import arena.junit.exec.ExecutableComponent;
import arena.junit.exec.ExecutableComponentBuilder;
import arena.junit.ffi.ArenaLogLevel;
import arena.examples.oauth.OauthClaims;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import arena.junit.oauth.OauthSigner;
import arena.junit.playbook.LocalstackModels;
import arena.junit.readiness.HttpReadinessCheck;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import org.junit.platform.suite.api.SelectClasses;
import org.junit.platform.suite.api.Suite;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import software.amazon.awssdk.auth.credentials.AwsBasicCredentials;
import software.amazon.awssdk.auth.credentials.StaticCredentialsProvider;
import software.amazon.awssdk.regions.Region;
import software.amazon.awssdk.services.sqs.SqsClient;
import software.amazon.awssdk.services.sqs.model.GetQueueUrlRequest;

@Suite
@SelectClasses({
  ReadingsCrudComponentTest.class,
  CalibrationOutageComponentTest.class,
  HttpPlaybookVerificationComponentTest.class,
  DeviceLifecycleComponentTest.class,
  WeatherCrudComponentTest.class
})
@Arena
public final class ComponentTestSuite {

  static final ObjectMapper MAPPER = new ObjectMapper();
  static final String CALIBRATION_VALIDATE_PATH = "/api/v1/validate";

  @ArenaLogger(level = ArenaLogLevel.DEBUG)
  static final Logger LOG = LoggerFactory.getLogger(ComponentTestSuite.class);

  private static final EphemeralTestRuntime RT = EphemeralTestRuntime.get();
  private static final int WEB_APP_PORT = RT.execWebAppPort;
  private static final int WEB_APP_2_PORT = EphemeralTestRuntime.ephemeralTcpPort();
  private static final int POSTGRES_PORT = RT.postgresPort;
  private static final int MSSQL_PORT = RT.mssqlPort;
  private static final int ORACLE_PORT = RT.oraclePort;
  private static final int CALIBRATION_HOST_PORT = RT.calibrationHostPort;
  private static final int LOCALSTACK_HOST_PORT = RT.localstackHostPort;
  private static final int TEMPORAL_GRPC_PORT = RT.temporalGrpcPort;
  private static final int TEMPORAL_UI_PORT = RT.temporalUiPort;
  private static final int SMTP_HOST_PORT = RT.smtpPort;
  private static final int SMTP_UI_PORT = RT.smtpUiPort;
  private static final String TEMPORAL_TARGET = "127.0.0.1:" + TEMPORAL_GRPC_PORT;
  private static final int OAUTH_PORT = RT.oauthPort;
  private static final String OAUTH_ISSUER = RT.oauthIssuer;
  private static final String OAUTH_COGNITO_POOL_ID = "us-east-1_exampleUsers";
  private static final String OAUTH_PROVIDER_ISSUER = OAUTH_ISSUER + "/" + OAUTH_COGNITO_POOL_ID;
  private static final String POSTGRES_DB_NAME = "readings_db";
  private static final String POSTGRES_DB_USER = "readings_user";
  private static final String POSTGRES_DB_PASS = "readings_password";
  private static final String MSSQL_DB_NAME = "validationDb";
  private static final String MSSQL_DB_USER = "sa";
  private static final String MSSQL_DB_PASS = "yourStrong(!)Password";
  private static final String ORACLE_DB_NAME = "FREEPDB1";
  private static final String ORACLE_DB_USER = "weather_user_" + RT.runSuffix.substring(0, 8);
  private static final String ORACLE_DB_PASS = "pw_" + RT.runSuffix.substring(8, 20);
  private static final String ORACLE_ADMIN_PASS = "pw_" + RT.runSuffix.substring(20, 32);
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

  private static final String MSSQL_JDBC_URL =
      SeedValidationReadingPlaybook.jdbcUrl(MSSQL_PORT, MSSQL_DB_NAME, MSSQL_DB_USER, MSSQL_DB_PASS);

  private static SqsClient sqsClient;
  private static String sqsQueueUrl;
  static long readingsDeviceId;

  @ArenaDependency(logs = false)
  static final OauthDependency OAUTH =
      new OauthDependencyBuilder("example-api-oauth")
          .withPort(OAUTH_PORT)
          .withListenIp("0.0.0.0")
          .withServerTlsPem(OAUTH_PEM.certificatePem(), OAUTH_PEM.privateKeyPem())
          .withMetadataBaseUrl(OAUTH_ISSUER)
          .withIssuerCognito(OAUTH_COGNITO_POOL_ID)
          .build();

  @ArenaDependency(logs = false)
  static final PostgresDependency POSTGRES =
      new PostgresDependencyBuilder("example-api-postgres")
          .withImage("14.20-trixie")
          .withPort(POSTGRES_PORT)
          .withDatabaseName(POSTGRES_DB_NAME)
          .withDatabaseUsername(POSTGRES_DB_USER)
          .withDatabasePassword(POSTGRES_DB_PASS)
          .withStartupSqlScripts(readSchema("instrument_reading_db_schema.sql"))
          .build();

  @ArenaDependency(logs = false)
  static final MssqlDependency MSSQL =
      new MssqlDependencyBuilder("example-api-mssql")
          .withPort(MSSQL_PORT)
          .withDatabaseName(MSSQL_DB_NAME)
          .withDatabaseUsername(MSSQL_DB_USER)
          .withDatabasePassword(MSSQL_DB_PASS)
          .withStartupSqlScripts(readSchema("validation_db_schema.sql"))
          .build();

  @ArenaDependency(logs = false)
  static final OracleDependency ORACLE =
      new OracleDependencyBuilder("example-api-oracle")
          .withPort(ORACLE_PORT)
          .withDatabaseUsername(ORACLE_DB_USER)
          .withDatabasePassword(ORACLE_DB_PASS)
          .withAdminPassword(ORACLE_ADMIN_PASS)
          .withStartupSqlScripts(readSchema("weather_db_schema.sql"))
          // Oracle container start times are inconsistent across CI runners, so a longer timeout is required.
          .withSqlReadinessTimeout(Duration.ofMinutes(2))
          .build();

  @ArenaDependency(logs = false)
  static final HttpDependency CALIBRATION =
      new HttpDependencyBuilder("example-api-calibration").withPort(CALIBRATION_HOST_PORT).build();

  @ArenaDependency(logs = false)
  static final LocalstackDependency LOCALSTACK = buildLocalstack();

  @ArenaDependency(logs = false)
  static final TemporalDependency TEMPORAL =
      new TemporalDependencyBuilder("example-api-temporal")
          .withImage("1.8.0")
          .withPort(TEMPORAL_GRPC_PORT)
          .withUiPort(TEMPORAL_UI_PORT)
          .build();

  @ArenaDependency(logs = false)
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

  @ArenaPlaybook(execOnDependencyStart = false)
  static final SeedValidationReadingPlaybook SEED_VALIDATION_READING =
      new SeedValidationReadingPlaybook(MSSQL_JDBC_URL);

  @ArenaPlaybook(execOnDependencyStart = false)
  static final ResetReadingsDbPlaybook RESET_READINGS_DB =
      new ResetReadingsDbPlaybook(POSTGRES.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final ResetWeatherDbPlaybook RESET_WEATHER_DB =
      new ResetWeatherDbPlaybook(ORACLE.identifier());

  @ArenaComponent(logs = true)
  static final ExecutableComponent WEB_APP = buildWebApp("example-api-web-app", WEB_APP_PORT);

  @ArenaComponent(logs = true)
  static final ExecutableComponent WEB_APP_2 = buildWebApp("example-api-web-app-2", WEB_APP_2_PORT);

  @ArenaAfterOpen
  static void afterOpen() throws Exception {
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

  private static ExecutableComponent buildWebApp(String name, int port) {
    String appLauncher;
    try {
      appLauncher = Runfiles.findWebAppLauncher();
    } catch (Exception e) {
      throw new IllegalStateException("failed to locate web app launcher runfile", e);
    }
    assertTrue(!appLauncher.isEmpty(), "web app launcher must be present under Bazel runfiles");
    return new ExecutableComponentBuilder(name)
        .withExecutablePath(appLauncher)
        .withEnvVar("WEB_APP_PORT", String.valueOf(port))
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
        .withEnvVar(
            "ORACLE_CONNECTION_STRING",
            ORACLE_DB_USER
                + "/"
                + ORACLE_DB_PASS
                + "@localhost:"
                + ORACLE_PORT
                + "/"
                + ORACLE_DB_NAME)
        .withEnvVar("TEMPORAL_TARGET", TEMPORAL_TARGET)
        .withEnvVar("SMTP_HOST", "127.0.0.1")
        .withEnvVar("SMTP_PORT", String.valueOf(SMTP_HOST_PORT))
        .withEnvVar("OAUTH_ISSUER_URL", OAUTH_PROVIDER_ISSUER)
        .withEnvVar("OAUTH_TLS_CA_FILE", OAUTH_CA_PATH)
        .withEnvVar("OAUTH_REQUIRED_ACCESS_TOKEN_SCOPES", "readings")
        .withEnvVar("AWS_ENDPOINT_URL", LOCALSTACK_ENDPOINT)
        .withEnvVar("AWS_DEFAULT_REGION", REGION)
        .withEnvVar("AWS_ACCESS_KEY_ID", AWS_DUMMY.get("aws_access_key_id"))
        .withEnvVar("AWS_SECRET_ACCESS_KEY", AWS_DUMMY.get("aws_secret_access_key"))
        .withEnvVar("EVENT_BUS_NAME", EVENT_BUS_NAME)
        .withEnvVar("EVENT_SOURCE", EVENT_SOURCE)
        .withReadinessCheck(
            HttpReadinessCheck.create(), "http://127.0.0.1:" + port + "/health", 30_000L)
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

  static String claimsWithScope(String scope) throws Exception {
    return OauthClaims.withScope(MAPPER, OAUTH_PROVIDER_ISSUER, scope);
  }

  static ApiClient apiClient() throws Exception {
    String token = OauthSigner.forFixture(ComponentTestSuite.class).sign(claimsWithScope("readings"));
    return new ApiClient("http://127.0.0.1:" + WEB_APP_PORT, token, MAPPER);
  }

  static ApiClient apiClient2() throws Exception {
    String token =
        OauthSigner.forFixture(ComponentTestSuite.class).sign(claimsWithScope("readings"));
    return new ApiClient("http://127.0.0.1:" + WEB_APP_2_PORT, token, MAPPER);
  }

  static String webAppBaseUrl() {
    return "http://127.0.0.1:" + WEB_APP_PORT;
  }

  static void waitDeviceProvisionedEmail(String needle) throws Exception {
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

  static JsonNode waitReadingCreatedOnQueue(int expectedId) throws Exception {
    return SqsWait.waitReadingCreatedDetail(MAPPER, sqsClient, sqsQueueUrl, expectedId);
  }

  static int seededValidationRowCount() throws Exception {
    return SeedValidationReadingPlaybook.countSeededRows(MSSQL_JDBC_URL);
  }
}
