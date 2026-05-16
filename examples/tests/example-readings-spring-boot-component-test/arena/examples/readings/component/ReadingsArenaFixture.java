package arena.examples.readings.component;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.ObjectMapper;
import arena.junit.ClosedArena;
import arena.junit.OpenArena;
import arena.junit.ffi.ArenaLogLevel;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.dep.LocalstackDependency;
import arena.junit.dep.LocalstackDependencyBuilder;
import arena.junit.dep.MssqlDependency;
import arena.junit.dep.MssqlDependencyBuilder;
import arena.junit.dep.PostgresDependency;
import arena.junit.dep.PostgresDependencyBuilder;
import arena.junit.exec.ExecutableComponentBuilder;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import arena.junit.playbook.LocalstackModels;
import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.playbook.ManagedHttpPlaybookBuilder;
import arena.junit.playbook.ManagedLocalstackPlaybook;
import arena.junit.readiness.HttpReadinessCheck;
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
import java.util.Map;
import java.util.UUID;
import javax.net.ssl.SSLContext;
import javax.net.ssl.TrustManagerFactory;
import arena.junit.playbook.ArenaSession;
import org.junit.jupiter.api.extension.BeforeAllCallback;
import org.junit.jupiter.api.extension.ExtensionContext;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class ReadingsArenaFixture implements BeforeAllCallback, ArenaSession {

  private static final Logger LOG = LoggerFactory.getLogger(ReadingsArenaFixture.class);

  static final ObjectMapper MAPPER = new ObjectMapper();

  private static final int WEB_APP_PORT = 3010;
  private static final int POSTGRES_PORT = 5560;
  private static final int MSSQL_PORT = 1438;
  private static final int CALIBRATION_HOST_PORT = 3011;
  private static final int LOCALSTACK_HOST_PORT = 4570;
  private static final int OAUTH_PORT = 9446;
  private static final String OAUTH_ISSUER = "https://127.0.0.1:" + OAUTH_PORT;
  private static final String POSTGRES_DB_NAME = "readings_db";
  private static final String POSTGRES_DB_USER = "readings_user";
  private static final String POSTGRES_DB_PASS = "readings_password";
  private static final String MSSQL_DB_NAME = "validationDb";
  private static final String MSSQL_DB_USER = "sa";
  private static final String MSSQL_DB_PASS = "yourStrong(!)Password";
  private static final String NETWORK_NAME = "arena-readings-api-network";
  private static final String CALIBRATION_VALIDATE_PATH = "/api/v1/validate";
  private static final String EVENT_BUS_NAME = "readings-api-events";
  private static final String EVENT_SOURCE = "readings.api";
  private static final String QUEUE_NAME = "readings-api-events-q";
  private static final String EVENT_RULE_NAME = "readings-api-rule";
  private static final String REGION = "us-east-1";
  private static final Map<String, String> AWS_DUMMY =
      Map.of("aws_access_key_id", "test", "aws_secret_access_key", "test");

  private OpenArena arena;
  private String oauthCaPath;
  private String accessToken;
  private String mssqlIdentifier;
  private String localstackEndpoint;
  private ManagedLocalstackPlaybook localstackSessionPb;

  @Override
  public OpenArena arena() {
    return arena;
  }

  public String accessToken() {
    return accessToken;
  }

  public String mssqlIdentifier() {
    return mssqlIdentifier;
  }

  public String localstackEndpoint() {
    return localstackEndpoint;
  }

  public ManagedLocalstackPlaybook localstackSessionPlaybook() {
    return localstackSessionPb;
  }

  public int webAppPort() {
    return WEB_APP_PORT;
  }

  public String region() {
    return REGION;
  }

  public String queueName() {
    return QUEUE_NAME;
  }

  public Map<String, String> awsDummyCredentials() {
    return AWS_DUMMY;
  }

  @Override
  public void beforeAll(ExtensionContext context) throws Exception {
    OauthLoopbackTls.PemPair pem = OauthLoopbackTls.oauthLoopbackTlsPemPair();
    Path ca = Files.createTempFile("readings-api-oauth-", ".pem");
    Files.writeString(ca, pem.certificatePem(), StandardCharsets.UTF_8);
    oauthCaPath = ca.toString();

    OauthDependency oauth =
        new OauthDependencyBuilder("readings-api-oauth")
            .withPort(OAUTH_PORT)
            .withListenIp("0.0.0.0")
            .withServerTlsPem(pem.certificatePem(), pem.privateKeyPem())
            .withMetadataBaseUrl(OAUTH_ISSUER)
            .build();

    String schemaPath = ReadingsRunfiles.findSchema("instrument_reading_db_schema.sql");
    String mssqlSchemaPath = ReadingsRunfiles.findSchema("validation_db_schema.sql");
    assertTrue(!schemaPath.isEmpty(), "instrument_reading_db_schema.sql");
    assertTrue(!mssqlSchemaPath.isEmpty(), "validation_db_schema.sql");
    List<String> pgSql = List.of(Files.readString(Path.of(schemaPath), StandardCharsets.UTF_8));
    List<String> mssqlSql =
        List.of(Files.readString(Path.of(mssqlSchemaPath), StandardCharsets.UTF_8));

    PostgresDependency postgres =
        new PostgresDependencyBuilder("readings-api-postgres")
            .withImage("14.20-trixie")
            .withPort(POSTGRES_PORT)
            .withDatabaseName(POSTGRES_DB_NAME)
            .withDatabaseUsername(POSTGRES_DB_USER)
            .withDatabasePassword(POSTGRES_DB_PASS)
            .withStartupSqlScripts(pgSql)
            .build();

    MssqlDependency mssql =
        new MssqlDependencyBuilder("readings-api-mssql")
            .withPort(MSSQL_PORT)
            .withDatabaseName(MSSQL_DB_NAME)
            .withDatabaseUsername(MSSQL_DB_USER)
            .withDatabasePassword(MSSQL_DB_PASS)
            .withStartupSqlScripts(mssqlSql)
            .build();
    mssqlIdentifier = mssql.identifier();

    HttpDependency calibration =
        new HttpDependencyBuilder("readings-api-calibration").withPort(CALIBRATION_HOST_PORT).build();

    ManagedHttpPlaybook calibrationPlaybook =
        new ManagedHttpPlaybookBuilder("readings-api-calibration-default", calibration.identifier())
            .withMapping("POST", CALIBRATION_VALIDATE_PATH, 200, Map.of("valid", true))
            .build();

    String lsId = "ls-readings-api-" + UUID.randomUUID().toString().substring(0, 8);
    LocalstackDependency localstack =
        new LocalstackDependencyBuilder(lsId)
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

    localstackSessionPb =
        new ManagedLocalstackPlaybook("readings-api-localstack-session", localstack.identifier());

    localstackEndpoint = "http://127.0.0.1:" + LOCALSTACK_HOST_PORT;

    String appLauncher = ReadingsRunfiles.findReadingsWebAppLauncher();
    assertTrue(!appLauncher.isEmpty(), "readings web app launcher must be present under Bazel runfiles");
    ExecutableComponentBuilder exec =
        new ExecutableComponentBuilder("readings-api-web-app")
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
            .withEnvVar("OAUTH_ISSUER_URL", OAUTH_ISSUER)
            .withEnvVar("OAUTH_TLS_CA_FILE", oauthCaPath)
            .withEnvVar("OAUTH_REQUIRED_ACCESS_TOKEN_SCOPES", "readings")
            .withEnvVar("AWS_ENDPOINT_URL", localstackEndpoint)
            .withEnvVar("AWS_DEFAULT_REGION", REGION)
            .withEnvVar("AWS_ACCESS_KEY_ID", AWS_DUMMY.get("aws_access_key_id"))
            .withEnvVar("AWS_SECRET_ACCESS_KEY", AWS_DUMMY.get("aws_secret_access_key"))
            .withEnvVar("EVENT_BUS_NAME", EVENT_BUS_NAME)
            .withEnvVar("EVENT_SOURCE", EVENT_SOURCE)
            .withReadinessCheck(
                HttpReadinessCheck.create(), "http://127.0.0.1:" + WEB_APP_PORT + "/health");

    Match match =
        new MatchBuilder("readings-api-happy-path")
            .withNetwork(NETWORK_NAME)
            .addDependency(oauth)
            .addDependency(postgres)
            .addDependency(mssql)
            .addDependency(calibration)
            .addDependency(localstack)
            .addComponent(exec.build())
            .registerPlaybook(calibrationPlaybook, true)
            .registerPlaybook(localstackSessionPb, true)
            .build();

    ClosedArena closed =
        new ClosedArena(
            "readings-api-arena",
            List.of(match),
            ArenaLogLevel.DEBUG,
            LOG,
            List.of("readings-api-web-app"),
            List.of(
                oauth.identifier(),
                postgres.identifier(),
                mssql.identifier(),
                calibration.identifier(),
                localstack.identifier()));
    arena = closed.open();
    fetchAccessToken();
  }

  public void stopReadingsArena() {
    if (arena != null) {
      arena.close();
      arena = null;
    }
  }

  private SSLContext sslContextFromPemFile(String path) throws Exception {
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

  private HttpClient oauthHttpClient() throws Exception {
    return HttpClient.newBuilder()
        .sslContext(sslContextFromPemFile(oauthCaPath))
        .connectTimeout(Duration.ofSeconds(30))
        .build();
  }

  private void fetchAccessToken() throws Exception {
    HttpClient c = oauthHttpClient();
    HttpResponse<String> disc =
        c.send(
            HttpRequest.newBuilder()
                .uri(URI.create(OAUTH_ISSUER + "/.well-known/oauth-authorization-server"))
                .GET()
                .timeout(Duration.ofSeconds(60))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(200, disc.statusCode(), disc.body());
    String tokenUrl = MAPPER.readTree(disc.body()).get("token_endpoint").asText();
    String form =
        "grant_type=client_credentials&client_id=arena-examples&scope=readings";
    HttpResponse<String> tok =
        c.send(
            HttpRequest.newBuilder()
                .uri(URI.create(tokenUrl))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .POST(HttpRequest.BodyPublishers.ofString(form))
                .timeout(Duration.ofSeconds(60))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(200, tok.statusCode(), tok.body());
    accessToken = MAPPER.readTree(tok.body()).get("access_token").asText();
  }
}
