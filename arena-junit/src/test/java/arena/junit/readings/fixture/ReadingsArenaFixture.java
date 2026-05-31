package arena.junit.readings.fixture;

import arena.junit.ClosedArena;
import arena.junit.ClosedArenaExtension;
import arena.junit.OpenArena;
import arena.junit.ffi.ArenaLogLevel;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.dep.KafkaDependency;
import arena.junit.dep.KafkaDependencyBuilder;
import arena.junit.dep.KafkaFlavor;
import arena.junit.dep.MssqlDependency;
import arena.junit.dep.MssqlDependencyBuilder;
import arena.junit.dep.PostgresDependency;
import arena.junit.dep.PostgresDependencyBuilder;
import arena.junit.exec.BuildTool;
import arena.junit.exec.ContainerizedComponentBuilder;
import arena.junit.exec.ExecutableComponentBuilder;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthIssuerHosts;
import arena.junit.oauth.OauthLoopbackTls;
import arena.junit.readiness.HttpReadinessCheck;
import arena.junit.readings.playbook.ReadingsCalibrationDefaultPlaybook;
import arena.junit.readings.playbook.ReadingsCalibrationOutagePlaybook;
import arena.junit.readings.playbook.ReadingsValidationDbPlaybook;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class ReadingsArenaFixture extends ClosedArenaExtension {

  private static final Logger LOG = LoggerFactory.getLogger(ReadingsArenaFixture.class);

  private static volatile Context CONTEXT;

  private String oauthCaPem;
  private String mssqlIdentifier;
  private String calibrationIdentifier;
  private boolean containerizedWebEnabled;

  public String oauthCaPem() {
    return CONTEXT.oauthCaPem;
  }

  public String accessToken() throws Exception {
    String t =
        ReadingsArenaConfig.fetchAccessToken(CONTEXT.oauthCaPem, CONTEXT.accessTokenCache);
    CONTEXT.accessTokenCache = t;
    return t;
  }

  public String mssqlIdentifier() {
    return CONTEXT.mssqlIdentifier;
  }

  public String calibrationIdentifier() {
    return CONTEXT.calibrationIdentifier;
  }

  public boolean containerizedWebEnabled() {
    return CONTEXT.containerizedWebEnabled;
  }

  @Override
  protected ClosedArena buildClosedArena() throws Exception {
    OauthLoopbackTls.PemPair pem = OauthLoopbackTls.oauthLoopbackTlsPemPair();
    oauthCaPem = pem.certificatePem();
    OauthDependency oauth =
        new OauthDependencyBuilder(ReadingsArenaConfig.DEP_NAME_OAUTH)
            .withPort(ReadingsArenaConfig.OAUTH_PORT)
            .withListenIp("0.0.0.0")
            .withServerTlsPem(pem.certificatePem(), pem.privateKeyPem())
            .withMetadataBaseUrl(ReadingsArenaConfig.OAUTH_ISSUER)
            .build();
    String schemaPath = ReadingsArenaConfig.findSchema("instrument_reading_db_schema.sql");
    String mssqlSchemaPath = ReadingsArenaConfig.findSchema("validation_db_schema.sql");
    List<String> pgSql =
        schemaPath.isEmpty() ? List.of() : List.of(Files.readString(Path.of(schemaPath)));
    List<String> mssqlSql =
        mssqlSchemaPath.isEmpty()
            ? List.of()
            : List.of(Files.readString(Path.of(mssqlSchemaPath)));
    PostgresDependency postgres =
        new PostgresDependencyBuilder(ReadingsArenaConfig.DEP_NAME_POSTGRES)
            .withImage(ReadingsArenaConfig.POSTGRES_IMAGE)
            .withPort(ReadingsArenaConfig.POSTGRES_PORT)
            .withDatabaseName(ReadingsArenaConfig.POSTGRES_DB_NAME)
            .withDatabaseUsername(ReadingsArenaConfig.POSTGRES_DB_USER)
            .withDatabasePassword(ReadingsArenaConfig.POSTGRES_DB_PASS)
            .withContainerName(ReadingsArenaConfig.POSTGRES_CONTAINER_NAME)
            .withStartupSqlScripts(pgSql)
            .build();
    KafkaDependency kafka =
        new KafkaDependencyBuilder(ReadingsArenaConfig.DEP_NAME_KAFKA)
            .withFlavor(KafkaFlavor.APACHE_NATIVE)
            .withPort(ReadingsArenaConfig.KAFKA_PORT)
            .withContainerName(ReadingsArenaConfig.KAFKA_CONTAINER_NAME)
            .withTopic(ReadingsArenaConfig.KAFKA_TOPIC)
            .build();
    MssqlDependency mssql =
        new MssqlDependencyBuilder(ReadingsArenaConfig.DEP_NAME_MSSQL)
            .withPort(ReadingsArenaConfig.MSSQL_PORT)
            .withDatabaseName(ReadingsArenaConfig.MSSQL_DB_NAME)
            .withDatabaseUsername(ReadingsArenaConfig.MSSQL_DB_USER)
            .withDatabasePassword(ReadingsArenaConfig.MSSQL_DB_PASS)
            .withContainerName(ReadingsArenaConfig.MSSQL_CONTAINER_NAME)
            .withStartupSqlScripts(mssqlSql)
            .build();
    mssqlIdentifier = mssql.identifier();
    HttpDependency calibration =
        new HttpDependencyBuilder(ReadingsArenaConfig.DEP_NAME_CALIBRATION_HTTP)
            .withPort(ReadingsArenaConfig.CALIBRATION_HOST_PORT)
            .withContainerName(ReadingsArenaConfig.CALIBRATION_CONTAINER_NAME)
            .build();
    calibrationIdentifier = calibration.identifier();
    ReadingsCalibrationDefaultPlaybook calibrationPlaybook =
        new ReadingsCalibrationDefaultPlaybook(calibration.identifier());
    ReadingsCalibrationOutagePlaybook calibrationOutagePlaybook =
        new ReadingsCalibrationOutagePlaybook(calibration.identifier());
    ReadingsValidationDbPlaybook validationDbPlaybook =
        new ReadingsValidationDbPlaybook(mssql.identifier());
    String bin = ReadingsArenaConfig.findAxumBinary();
    boolean bazel = System.getenv("RUNFILES_DIR") != null;
    ExecutableComponentBuilder execB =
        new ExecutableComponentBuilder(ReadingsArenaConfig.COMPONENT_NAME_EXECUTABLE)
            .withExecutablePath(bin)
            .withEnvVar("RUST_LOG", "info")
            .withEnvVar("OAUTH_TLS_CA_PEM", oauthCaPem)
            .withRuntimeArg("web_app_port", String.valueOf(ReadingsArenaConfig.EXEC_WEB_APP_PORT))
            .withRuntimeArg("postgres_connection_string", ReadingsArenaConfig.postgresConnectionLocal())
            .withRuntimeArg("kafka_bootstrap", "localhost:" + ReadingsArenaConfig.KAFKA_PORT)
            .withRuntimeArg(
                "calibration_url", "http://127.0.0.1:" + ReadingsArenaConfig.CALIBRATION_HOST_PORT)
            .withRuntimeArg("mssql_connection_string", ReadingsArenaConfig.mssqlConnectionLocal())
            .withRuntimeArg("oauth_issuer_url", ReadingsArenaConfig.OAUTH_ISSUER)
            .withReadinessCheck(
                HttpReadinessCheck.create(),
                "http://127.0.0.1:" + ReadingsArenaConfig.EXEC_WEB_APP_PORT + "/health");
    if (!bazel) {
      execB = execB.withSourcePath("examples").withBuildTool(BuildTool.CARGO);
    }
    var exec = execB.build();
    MatchBuilder mb =
        new MatchBuilder(ReadingsArenaConfig.MATCH_NAME)
            .withNetwork(ReadingsArenaConfig.NETWORK_NAME)
            .addDependency(oauth)
            .addDependency(postgres)
            .addDependency(kafka)
            .addDependency(mssql)
            .addDependency(calibration)
            .addComponent(exec)
            .registerPlaybook(calibrationPlaybook, true)
            .registerPlaybook(calibrationOutagePlaybook, false)
            .registerPlaybook(validationDbPlaybook, false);
    containerizedWebEnabled = false;
    if (OauthIssuerHosts.oauthIssuerHostIsNonLoopback()) {
      Path ctxPath = ReadingsArenaConfig.prepareDockerImageContext(bin);
      if (ctxPath != null) {
        var container =
            new ContainerizedComponentBuilder(
                    ReadingsArenaConfig.COMPONENT_NAME_CONTAINERIZED, "Containerfile")
                .withBuildContext(ctxPath.toString())
                .withImageTag(ReadingsArenaConfig.DOCKER_IMAGE_TAG)
                .withNetwork(ReadingsArenaConfig.NETWORK_NAME)
                .withPortMapping(ReadingsArenaConfig.DOCKER_WEB_HOST_PORT, 3000)
                .withHostMapping("host.docker.internal:host-gateway")
                .withEnvVar("RUST_LOG", "info")
                .withEnvVar("OAUTH_TLS_CA_PEM", oauthCaPem)
                .withRuntimeArg("web_app_port", "3000")
                .withRuntimeArg("postgres_connection_string", ReadingsArenaConfig.postgresConnectionDocker())
                .withRuntimeArg(
                    "kafka_bootstrap",
                    ReadingsArenaConfig.kafkaBootstrapDocker(
                        ReadingsArenaConfig.KAFKA_CONTAINER_NAME,
                        KafkaDependency.KAFKA_INTERNAL_DOCKER_PORT))
                .withRuntimeArg(
                    "calibration_url",
                    "http://" + ReadingsArenaConfig.CALIBRATION_CONTAINER_NAME + ":8080")
                .withRuntimeArg("mssql_connection_string", ReadingsArenaConfig.mssqlConnectionDocker())
                .withRuntimeArg("oauth_issuer_url", ReadingsArenaConfig.OAUTH_ISSUER)
                .withReadinessCheck(
                    HttpReadinessCheck.create(),
                    "http://127.0.0.1:" + ReadingsArenaConfig.DOCKER_WEB_HOST_PORT + "/health")
                .build();
        mb = mb.addComponent(container);
        containerizedWebEnabled = true;
      }
    }
    Match match = mb.build();
    return new ClosedArena(
        ReadingsArenaConfig.CLOSED_ARENA_NAME,
        List.of(match),
        ArenaLogLevel.DEBUG,
        LOG,
        List.of(
            ReadingsArenaConfig.COMPONENT_NAME_EXECUTABLE,
            ReadingsArenaConfig.COMPONENT_NAME_CONTAINERIZED),
        List.of(
            ReadingsArenaConfig.DEP_NAME_OAUTH,
            ReadingsArenaConfig.DEP_NAME_POSTGRES,
            ReadingsArenaConfig.DEP_NAME_KAFKA,
            ReadingsArenaConfig.DEP_NAME_MSSQL,
            ReadingsArenaConfig.DEP_NAME_CALIBRATION_HTTP));
  }

  @Override
  protected void afterOpen(OpenArena openArena) throws Exception {
    CONTEXT =
        new Context(
            openArena,
            oauthCaPem,
            mssqlIdentifier,
            calibrationIdentifier,
            containerizedWebEnabled);
    CONTEXT.accessTokenCache = ReadingsArenaConfig.fetchAccessToken(CONTEXT.oauthCaPem, null);
  }

  @Override
  protected void beforeClose(OpenArena openArena) {
    CONTEXT = null;
  }

  static final class Context {
    final OpenArena openArena;
    final String oauthCaPem;
    final String mssqlIdentifier;
    final String calibrationIdentifier;
    final boolean containerizedWebEnabled;
    volatile String accessTokenCache;

    Context(
        OpenArena openArena,
        String oauthCaPem,
        String mssqlIdentifier,
        String calibrationIdentifier,
        boolean containerizedWebEnabled) {
      this.openArena = openArena;
      this.oauthCaPem = oauthCaPem;
      this.mssqlIdentifier = mssqlIdentifier;
      this.calibrationIdentifier = calibrationIdentifier;
      this.containerizedWebEnabled = containerizedWebEnabled;
    }
  }
}
