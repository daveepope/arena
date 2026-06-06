package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.playbooks.CalibrationApiErrorPathPlaybook;
import arena.examples.playbooks.CalibrationApiFlakyPlaybook;
import arena.examples.playbooks.CalibrationApiHappyPathPlaybook;
import arena.examples.playbooks.ResetValidationDbPlaybook;
import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.ClosedArena;
import arena.junit.ClosedArenaExtension;
import arena.junit.OpenArena;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.dep.KafkaDependency;
import arena.junit.dep.KafkaDependencyBuilder;
import arena.junit.dep.KafkaFlavor;
import arena.junit.dep.MssqlDependency;
import arena.junit.dep.MssqlDependencyBuilder;
import arena.junit.dep.PostgresDependency;
import arena.junit.dep.PostgresDependencyBuilder;
import arena.junit.exec.ExecutableComponentBuilder;
import arena.junit.ffi.ArenaLogLevel;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import arena.junit.readiness.HttpReadinessCheck;
import com.fasterxml.jackson.databind.ObjectMapper;
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
import java.util.Collection;
import java.util.List;
import javax.net.ssl.SSLContext;
import javax.net.ssl.TrustManagerFactory;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class ArenaFixture extends ClosedArenaExtension {

  private static final Logger LOG = LoggerFactory.getLogger(ArenaFixture.class);

  static final ObjectMapper MAPPER = new ObjectMapper();

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
  static final String CALIBRATION_VALIDATE_PATH = "/api/v1/validate";

  private String oauthCaPem;
  private String accessToken;

  public String accessToken() {
    return accessToken;
  }

  public int webAppPort() {
    return WEB_APP_PORT;
  }

  public int kafkaPort() {
    return KAFKA_PORT;
  }

  public String kafkaTopic() {
    return KAFKA_TOPIC;
  }

  @Override
  protected ClosedArena buildClosedArena() throws Exception {
    OauthLoopbackTls.PemPair pem = OauthLoopbackTls.oauthLoopbackTlsPemPair();
    oauthCaPem = pem.certificatePem();
    OauthDependency oauth =
        new OauthDependencyBuilder("example-api-oauth")
            .withPort(OAUTH_PORT)
            .withListenIp("0.0.0.0")
            .withServerTlsPem(pem.certificatePem(), pem.privateKeyPem())
            .withMetadataBaseUrl(OAUTH_ISSUER)
            .build();

    String schemaPath = Runfiles.findSchema("instrument_reading_db_schema.sql");
    String mssqlSchemaPath = Runfiles.findSchema("validation_db_schema.sql");
    assertTrue(!schemaPath.isEmpty(), "instrument_reading_db_schema.sql");
    assertTrue(!mssqlSchemaPath.isEmpty(), "validation_db_schema.sql");
    List<String> pgSql = List.of(Files.readString(Path.of(schemaPath), StandardCharsets.UTF_8));
    List<String> mssqlSql =
        List.of(Files.readString(Path.of(mssqlSchemaPath), StandardCharsets.UTF_8));

    PostgresDependency postgres =
        new PostgresDependencyBuilder("example-api-postgres")
            .withImage("14.20-trixie")
            .withPort(POSTGRES_PORT)
            .withDatabaseName(POSTGRES_DB_NAME)
            .withDatabaseUsername(POSTGRES_DB_USER)
            .withDatabasePassword(POSTGRES_DB_PASS)
            .withStartupSqlScripts(pgSql)
            .build();

    KafkaDependency kafka =
        new KafkaDependencyBuilder("example-api-kafka")
            .withFlavor(KafkaFlavor.APACHE_NATIVE)
            .withPort(KAFKA_PORT)
            .withTopic(KAFKA_TOPIC)
            .build();

    MssqlDependency mssql =
        new MssqlDependencyBuilder("example-api-mssql")
            .withPort(MSSQL_PORT)
            .withDatabaseName(MSSQL_DB_NAME)
            .withDatabaseUsername(MSSQL_DB_USER)
            .withDatabasePassword(MSSQL_DB_PASS)
            .withStartupSqlScripts(mssqlSql)
            .build();

    HttpDependency calibration =
        new HttpDependencyBuilder("example-api-calibration").withPort(CALIBRATION_HOST_PORT).build();

    CalibrationApiHappyPathPlaybook calibrationPlaybook =
        new CalibrationApiHappyPathPlaybook(calibration.identifier());
    CalibrationApiErrorPathPlaybook calibrationApiErrorPathPlaybook =
        new CalibrationApiErrorPathPlaybook(calibration.identifier());
    CalibrationApiFlakyPlaybook calibrationApiFlakyPlaybook =
        new CalibrationApiFlakyPlaybook(calibration.identifier());
    ResetValidationDbPlaybook resetValidationDbPlaybook =
        new ResetValidationDbPlaybook(mssql.identifier());

    String bin = Runfiles.findAxumBinary();
    assertTrue(!bin.isEmpty(), "example-readings-axum-web-app must be present under Bazel runfiles");

    ExecutableComponentBuilder exec =
        new ExecutableComponentBuilder("example-api-web-app")
            .withExecutablePath(bin)
            .withEnvVar("RUST_LOG", "info")
            .withEnvVar("OAUTH_TLS_CA_PEM", oauthCaPem)
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
                30_000L);

    Match match =
        new MatchBuilder(RT.containerName("example-api-happy-path"))
            .addDependency(oauth)
            .addDependency(postgres)
            .addDependency(kafka)
            .addDependency(mssql)
            .addDependency(calibration)
            .addComponent(exec.build())
            .registerPlaybook(calibrationPlaybook, true)
            .registerPlaybook(calibrationApiErrorPathPlaybook, false)
            .registerPlaybook(calibrationApiFlakyPlaybook, false)
            .registerPlaybook(resetValidationDbPlaybook, false)
            .build();

    return new ClosedArena(
        RT.containerName("example-api-arena"),
        List.of(match),
        ArenaLogLevel.WARN,
        LOG,
        List.of("example-api-web-app"),
        List.of(
            oauth.identifier(),
            postgres.identifier(),
            kafka.identifier(),
            mssql.identifier(),
            calibration.identifier()));
  }

  @Override
  protected void afterOpen(OpenArena openArena) throws Exception {
    fetchAccessToken();
  }

  private SSLContext sslContextFromPem(String pem) throws Exception {
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

  private void fetchAccessToken() throws Exception {
    HttpClient c =
        HttpClient.newBuilder()
            .sslContext(sslContextFromPem(oauthCaPem))
            .connectTimeout(Duration.ofSeconds(30))
            .build();
    HttpResponse<String> disc =
        c.send(
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
        c.send(
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
}
