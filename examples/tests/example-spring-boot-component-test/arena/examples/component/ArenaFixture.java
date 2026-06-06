package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.ObjectMapper;
import arena.examples.playbooks.CalibrationApiHappyPathPlaybook;
import arena.examples.playbooks.CalibrationApiErrorPathPlaybook;
import arena.examples.playbooks.CalibrationApiFlakyPlaybook;
import arena.examples.playbooks.EventsPurgePlaybook;
import arena.examples.playbooks.ResetValidationDbPlaybook;
import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.ClosedArena;
import arena.junit.ClosedArenaExtension;
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
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import software.amazon.awssdk.auth.credentials.AwsBasicCredentials;
import software.amazon.awssdk.auth.credentials.StaticCredentialsProvider;
import software.amazon.awssdk.regions.Region;
import software.amazon.awssdk.services.sqs.SqsClient;
import software.amazon.awssdk.services.sqs.model.GetQueueUrlRequest;

public final class ArenaFixture extends ClosedArenaExtension {

  private static final Logger LOG = LoggerFactory.getLogger(ArenaFixture.class);

  static final ObjectMapper MAPPER = new ObjectMapper();

  private static final EphemeralTestRuntime RT = EphemeralTestRuntime.get();
  private static final int WEB_APP_PORT = RT.execWebAppPort;
  private static final int POSTGRES_PORT = RT.postgresPort;
  private static final int MSSQL_PORT = RT.mssqlPort;
  private static final int CALIBRATION_HOST_PORT = RT.calibrationHostPort;
  private static final int LOCALSTACK_HOST_PORT = RT.localstackHostPort;
  private static final int OAUTH_PORT = RT.oauthPort;
  private static final String OAUTH_ISSUER = RT.oauthIssuer;
  private static final String POSTGRES_DB_NAME = "readings_db";
  private static final String POSTGRES_DB_USER = "readings_user";
  private static final String POSTGRES_DB_PASS = "readings_password";
  private static final String MSSQL_DB_NAME = "validationDb";
  private static final String MSSQL_DB_USER = "sa";
  private static final String MSSQL_DB_PASS = "yourStrong(!)Password";
  private static final String NETWORK_NAME = RT.networkName("arena-example-api-network");
  static final String CALIBRATION_VALIDATE_PATH = "/api/v1/validate";
  private static final String EVENT_BUS_NAME = "example-api-events";
  private static final String EVENT_SOURCE = "readings.api";
  private static final String QUEUE_NAME = "example-api-events-q";
  private static final String EVENT_RULE_NAME = "example-api-rule";
  private static final String REGION = "us-east-1";
  private static final Map<String, String> AWS_DUMMY =
      Map.of("aws_access_key_id", "test", "aws_secret_access_key", "test");

  private String oauthCaPath;
  private String accessToken;
  private String localstackEndpoint;
  private SqsClient sqsClient;
  private String sqsQueueUrl;

  public String accessToken() {
    return accessToken;
  }

  public String localstackEndpoint() {
    return localstackEndpoint;
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

  public SqsClient sqsClient() {
    return sqsClient;
  }

  public String sqsQueueUrl() {
    return sqsQueueUrl;
  }

  @Override
  protected ClosedArena buildClosedArena() throws Exception {
    OauthLoopbackTls.PemPair pem = OauthLoopbackTls.oauthLoopbackTlsPemPair();
    Path ca = Files.createTempFile("example-api-oauth-", ".pem");
    Files.writeString(ca, pem.certificatePem(), StandardCharsets.UTF_8);
    oauthCaPath = ca.toString();

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

    String lsId = "ls-example-api-" + UUID.randomUUID().toString().substring(0, 8);
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

    EventsPurgePlaybook eventsPurgePlaybook =
        new EventsPurgePlaybook(localstack.identifier());

    localstackEndpoint = "http://127.0.0.1:" + LOCALSTACK_HOST_PORT;

    String appLauncher = Runfiles.findWebAppLauncher();
    assertTrue(!appLauncher.isEmpty(), "web app launcher must be present under Bazel runfiles");
    ExecutableComponentBuilder exec =
        new ExecutableComponentBuilder("example-api-web-app")
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
        new MatchBuilder("example-api-happy-path")
            .withNetwork(NETWORK_NAME)
            .addDependency(oauth)
            .addDependency(postgres)
            .addDependency(mssql)
            .addDependency(calibration)
            .addDependency(localstack)
            .addComponent(exec.build())
            .registerPlaybook(calibrationPlaybook, true)
            .registerPlaybook(calibrationApiErrorPathPlaybook, false)
            .registerPlaybook(calibrationApiFlakyPlaybook, false)
            .registerPlaybook(eventsPurgePlaybook, true)
            .registerPlaybook(resetValidationDbPlaybook, false)
            .build();

    ClosedArena closed =
        new ClosedArena(
            "example-api-arena",
            List.of(match),
            ArenaLogLevel.WARN,
            LOG,
            List.of("example-api-web-app"),
            List.of(
                oauth.identifier(),
                postgres.identifier(),
                mssql.identifier(),
                calibration.identifier(),
                localstack.identifier()));
    return closed;
  }

  @Override
  protected void afterOpen(OpenArena openArena) throws Exception {
    fetchAccessToken();
    var creds =
        StaticCredentialsProvider.create(
            AwsBasicCredentials.create(
                AWS_DUMMY.get("aws_access_key_id"), AWS_DUMMY.get("aws_secret_access_key")));
    sqsClient =
        SqsClient.builder()
            .region(Region.of(REGION))
            .endpointOverride(URI.create(localstackEndpoint))
            .credentialsProvider(creds)
            .build();
    sqsQueueUrl =
        sqsClient
            .getQueueUrl(GetQueueUrlRequest.builder().queueName(QUEUE_NAME).build())
            .queueUrl();
  }

  @Override
  protected void beforeClose(OpenArena openArena) {
    if (sqsClient != null) {
      sqsClient.close();
      sqsClient = null;
    }
    sqsQueueUrl = null;
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
                .timeout(Duration.ofSeconds(30))
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
                .timeout(Duration.ofSeconds(30))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(200, tok.statusCode(), tok.body());
    accessToken = MAPPER.readTree(tok.body()).get("access_token").asText();
  }
}
