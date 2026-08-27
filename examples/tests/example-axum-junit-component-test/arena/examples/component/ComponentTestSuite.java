package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.http.ApiClient;
import arena.examples.playbooks.CalibrationApiErrorPathPlaybook;
import arena.examples.playbooks.CalibrationApiFlakyPlaybook;
import arena.examples.playbooks.CalibrationApiHappyPathPlaybook;
import arena.examples.playbooks.ResetValidationDbPlaybook;
import arena.examples.playbooks.SeedValidationReadingPlaybook;
import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.Arena;
import arena.junit.ArenaComponent;
import arena.junit.ArenaDependency;
import arena.junit.ArenaLogger;
import arena.junit.ArenaPlaybook;
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
import arena.junit.ffi.ArenaLogLevel;
import arena.examples.oauth.OauthClaims;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import arena.junit.oauth.OauthSigner;
import arena.junit.readiness.HttpReadinessCheck;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.junit.platform.suite.api.SelectClasses;
import org.junit.platform.suite.api.Suite;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

@Suite
@SelectClasses({
  ReadingsCrudComponentTest.class,
  CalibrationOutageComponentTest.class,
  HttpPlaybookVerificationComponentTest.class
})
@Arena
public final class ComponentTestSuite {

  static final ObjectMapper MAPPER = new ObjectMapper();
  static final String CALIBRATION_VALIDATE_PATH = "/api/v1/validate";

  @ArenaLogger(level = ArenaLogLevel.DEBUG)
  static final Logger LOG = LoggerFactory.getLogger(ComponentTestSuite.class);

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

  private static final String MSSQL_JDBC_URL =
      SeedValidationReadingPlaybook.jdbcUrl(MSSQL_PORT, MSSQL_DB_NAME, MSSQL_DB_USER, MSSQL_DB_PASS);

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

  @ArenaPlaybook(execOnDependencyStart = false)
  static final SeedValidationReadingPlaybook SEED_VALIDATION_READING =
      new SeedValidationReadingPlaybook(MSSQL_JDBC_URL);

  @ArenaComponent(logs = true)
  static final ExecutableComponent WEB_APP = buildWebApp();

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

  static String claimsWithScope(String scope) throws Exception {
    return OauthClaims.withScope(MAPPER, OAUTH_ISSUER, scope);
  }

  static ApiClient apiClient() throws Exception {
    String token = OauthSigner.forFixture(ComponentTestSuite.class).sign(claimsWithScope("readings"));
    return new ApiClient("http://127.0.0.1:" + WEB_APP_PORT, token, MAPPER);
  }

  static JsonNode waitReadingCreatedOnKafka(KafkaWait.ReadingCreateAction create) throws Exception {
    return KafkaWait.waitReadingCreatedDetail(MAPPER, "localhost:" + KAFKA_PORT, KAFKA_TOPIC, create);
  }

  static int seededValidationRowCount() throws Exception {
    return SeedValidationReadingPlaybook.countSeededRows(MSSQL_JDBC_URL);
  }
}
