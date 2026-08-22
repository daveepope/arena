package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.http.ApiClient;
import arena.examples.playbooks.CalibrationApiHappyPathPlaybook;
import arena.examples.playbooks.EventsPurgePlaybook;
import arena.examples.playbooks.ResetReadingsDbPlaybook;
import arena.examples.playbooks.ResetWeatherDbPlaybook;
import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.Arena;
import arena.junit.ArenaAfterOpen;
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
import arena.junit.match.ArenaRunnableComponent;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import arena.junit.playbook.LocalstackModels;
import arena.junit.readiness.HttpReadinessCheck;
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
import org.junit.platform.suite.api.SelectClasses;
import org.junit.platform.suite.api.Suite;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

@Suite
@SelectClasses({ChainedDeviceLifecycleComponentTest.class})
@Arena
public final class ChainedComponentTestSuite {

  static final ObjectMapper MAPPER = new ObjectMapper();

  @ArenaLogger(level = ArenaLogLevel.DEBUG)
  static final Logger LOG = LoggerFactory.getLogger(ChainedComponentTestSuite.class);

  private static final EphemeralTestRuntime RT = EphemeralTestRuntime.get();
  private static final int WEB_APP_PORT = RT.execWebAppPort;
  private static final int WEB_APP_CHILD_PORT = EphemeralTestRuntime.ephemeralTcpPort();
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
  private static final String POSTGRES_DB_NAME = "readings_db";
  private static final String POSTGRES_DB_USER = "readings_user";
  private static final String POSTGRES_DB_PASS = "readings_password";
  private static final String MSSQL_DB_NAME = "validationDb";
  private static final String MSSQL_DB_USER = "sa";
  private static final String MSSQL_DB_PASS = "yourStrong(!)Password";
  private static final String ORACLE_DB_NAME = "weatherdb";
  private static final String ORACLE_DB_USER = "weather_user_" + RT.runSuffix.substring(0, 8);
  private static final String ORACLE_DB_PASS = "pw_" + RT.runSuffix.substring(8, 20);
  private static final String ORACLE_ADMIN_PASS = "pw_" + RT.runSuffix.substring(20, 32);
  private static final String EVENT_BUS_NAME = "example-api-chained-events";
  private static final String EVENT_SOURCE = "readings.api";
  private static final String QUEUE_NAME = "example-api-chained-events-q";
  private static final String EVENT_RULE_NAME = "example-api-chained-rule";
  private static final String REGION = "us-east-1";
  private static final Map<String, String> AWS_DUMMY =
      Map.of("aws_access_key_id", "test", "aws_secret_access_key", "test");
  private static final String LOCALSTACK_ENDPOINT = "http://127.0.0.1:" + LOCALSTACK_HOST_PORT;

  private static final OauthLoopbackTls.PemPair OAUTH_PEM =
      OauthLoopbackTls.oauthLoopbackTlsPemPair();
  private static final String OAUTH_CA_PATH = writeOauthCaPemFile();

  private static String accessToken;

  @ArenaDependency(logs = false)
  static final OauthDependency OAUTH =
      new OauthDependencyBuilder("example-api-chained-oauth")
          .withPort(OAUTH_PORT)
          .withListenIp("0.0.0.0")
          .withServerTlsPem(OAUTH_PEM.certificatePem(), OAUTH_PEM.privateKeyPem())
          .withMetadataBaseUrl(OAUTH_ISSUER)
          .build();

  private static final PostgresDependency POSTGRES =
      new PostgresDependencyBuilder("example-api-chained-postgres")
          .withImage("14.20-trixie")
          .withPort(POSTGRES_PORT)
          .withDatabaseName(POSTGRES_DB_NAME)
          .withDatabaseUsername(POSTGRES_DB_USER)
          .withDatabasePassword(POSTGRES_DB_PASS)
          .withStartupSqlScripts(readSchema("instrument_reading_db_schema.sql"))
          .build();

  @ArenaDependency(logs = false)
  static final MssqlDependency MSSQL =
      new MssqlDependencyBuilder("example-api-chained-mssql")
          .withPort(MSSQL_PORT)
          .withDatabaseName(MSSQL_DB_NAME)
          .withDatabaseUsername(MSSQL_DB_USER)
          .withDatabasePassword(MSSQL_DB_PASS)
          .withStartupSqlScripts(readSchema("validation_db_schema.sql"))
          .build();

  @ArenaDependency(logs = false)
  static final OracleDependency ORACLE =
      new OracleDependencyBuilder("example-api-chained-oracle")
          .withPort(ORACLE_PORT)
          .withDatabaseName(ORACLE_DB_NAME)
          .withDatabaseUsername(ORACLE_DB_USER)
          .withDatabasePassword(ORACLE_DB_PASS)
          .withAdminPassword(ORACLE_ADMIN_PASS)
          .withStartupSqlScripts(readSchema("weather_db_schema.sql"))
          .build();

  @ArenaDependency(logs = false)
  static final HttpDependency CALIBRATION =
      new HttpDependencyBuilder("example-api-chained-calibration")
          .withPort(CALIBRATION_HOST_PORT)
          .build();

  @ArenaDependency(logs = false)
  static final LocalstackDependency LOCALSTACK = buildLocalstack();

  @ArenaDependency(logs = false)
  static final TemporalDependency TEMPORAL =
      new TemporalDependencyBuilder("example-api-chained-temporal")
          .withImage("1.8.0")
          .withPort(TEMPORAL_GRPC_PORT)
          .withUiPort(TEMPORAL_UI_PORT)
          .addChildDependency(POSTGRES)
          .build();

  @ArenaDependency(logs = false)
  static final SmtpDependency SMTP =
      new SmtpDependencyBuilder("example-api-chained-smtp")
          .withPort(SMTP_HOST_PORT)
          .withUiPort(SMTP_UI_PORT)
          .withStarttls()
          .build();

  @ArenaPlaybook
  static final CalibrationApiHappyPathPlaybook CALIBRATION_HAPPY_PATH =
      new CalibrationApiHappyPathPlaybook(CALIBRATION.identifier());

  @ArenaPlaybook static final EventsPurgePlaybook EVENTS_PURGE = new EventsPurgePlaybook(LOCALSTACK.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final ResetReadingsDbPlaybook RESET_READINGS_DB =
      new ResetReadingsDbPlaybook(POSTGRES.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final ResetWeatherDbPlaybook RESET_WEATHER_DB =
      new ResetWeatherDbPlaybook(ORACLE.identifier());

  private static final ExecutableComponent WEB_APP_CHILD =
      buildWebApp("example-api-chained-web-app-child", WEB_APP_CHILD_PORT, List.of());

  @ArenaComponent(logs = true)
  static final ExecutableComponent WEB_APP =
      buildWebApp("example-api-chained-web-app", WEB_APP_PORT, List.of(WEB_APP_CHILD));

  @ArenaAfterOpen
  static void afterOpen() throws Exception {
    fetchAccessToken();
  }

  private static String writeOauthCaPemFile() {
    try {
      Path ca = Files.createTempFile("example-api-chained-oauth-", ".pem");
      Files.writeString(ca, OAUTH_PEM.certificatePem(), StandardCharsets.UTF_8);
      return ca.toString();
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }

  private static LocalstackDependency buildLocalstack() {
    String lsId = "ls-example-api-chained-" + UUID.randomUUID().toString().substring(0, 8);
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

  private static ExecutableComponent buildWebApp(
      String name, int port, List<ArenaRunnableComponent> children) {
    String appLauncher;
    try {
      appLauncher = Runfiles.findWebAppLauncher();
    } catch (Exception e) {
      throw new IllegalStateException("failed to locate web app launcher runfile", e);
    }
    assertTrue(!appLauncher.isEmpty(), "web app launcher must be present under Bazel runfiles");
    ExecutableComponentBuilder builder =
        new ExecutableComponentBuilder(name)
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
                HttpReadinessCheck.create(), "http://127.0.0.1:" + port + "/health", 30_000L);
    for (ArenaRunnableComponent child : children) {
      builder.addChildComponent(child);
    }
    return builder.build();
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

  static ApiClient apiClient() {
    return new ApiClient("http://127.0.0.1:" + WEB_APP_PORT, accessToken, MAPPER);
  }

  static ApiClient apiClient2() {
    return new ApiClient("http://127.0.0.1:" + WEB_APP_CHILD_PORT, accessToken, MAPPER);
  }
}
