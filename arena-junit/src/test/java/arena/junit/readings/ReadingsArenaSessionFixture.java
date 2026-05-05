package arena.junit.readings;

import arena.junit.ClosedArena;
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
import arena.junit.exec.BuildTool;
import arena.junit.exec.ContainerizedComponentBuilder;
import arena.junit.exec.ExecutableComponentBuilder;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthIssuerHosts;
import arena.junit.oauth.OauthLoopbackTls;
import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.playbook.ManagedHttpPlaybookBuilder;
import arena.junit.readiness.HttpReadinessCheck;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import arena.junit.playbook.ArenaSession;
import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.jupiter.api.extension.BeforeAllCallback;
import org.junit.jupiter.api.extension.ExtensionContext;

public final class ReadingsArenaSessionFixture implements BeforeAllCallback, AfterAllCallback, ArenaSession {

  private static final ExtensionContext.Namespace NS =
      ExtensionContext.Namespace.create("arena.junit.readings.session");

  private static volatile Session SESSION;

  private static final class Holder {
    Session session;
    int refs;
  }

  @Override
  public OpenArena arena() {
    return SESSION.arena;
  }

  public String oauthCaPem() {
    return SESSION.oauthCaPem;
  }

  public String accessToken() throws Exception {
    String t =
        ReadingsArenaConfig.fetchAccessToken(SESSION.oauthCaPem, SESSION.accessTokenCache);
    SESSION.accessTokenCache = t;
    return t;
  }

  public String mssqlIdentifier() {
    return SESSION.mssqlIdentifier;
  }

  public String calibrationIdentifier() {
    return SESSION.calibrationIdentifier;
  }

  public boolean containerizedWebEnabled() {
    return SESSION.containerizedWebEnabled;
  }

  @Override
  public void beforeAll(ExtensionContext context) throws Exception {
    ExtensionContext root = context.getRoot();
    ExtensionContext.Store store = root.getStore(NS);
    Holder h =
        (Holder)
            store.getOrComputeIfAbsent(
                "holder",
                k -> new Holder());
    synchronized (h) {
      if (h.session == null) {
        h.session = Session.open();
      }
      h.refs++;
      SESSION = h.session;
    }
  }

  @Override
  public void afterAll(ExtensionContext context) {
    ExtensionContext root = context.getRoot();
    ExtensionContext.Store store = root.getStore(NS);
    Holder h = store.get("holder", Holder.class);
    if (h == null) {
      return;
    }
    synchronized (h) {
      h.refs--;
      if (h.refs == 0 && h.session != null) {
        h.session.close();
        h.session = null;
        SESSION = null;
      }
    }
  }

  static final class Session {
    final OpenArena arena;
    final String oauthCaPem;
    final String mssqlIdentifier;
    final String calibrationIdentifier;
    volatile String accessTokenCache;
    final boolean containerizedWebEnabled;

    private Session(
        OpenArena arena,
        String oauthCaPem,
        String mssqlIdentifier,
        String calibrationIdentifier,
        boolean containerizedWebEnabled) {
      this.arena = arena;
      this.oauthCaPem = oauthCaPem;
      this.mssqlIdentifier = mssqlIdentifier;
      this.calibrationIdentifier = calibrationIdentifier;
      this.containerizedWebEnabled = containerizedWebEnabled;
    }

    static Session open() throws Exception {
      OauthLoopbackTls.PemPair pem = OauthLoopbackTls.oauthLoopbackTlsPemPair();
      String oauthCa = pem.certificatePem();
      OauthDependency oauth =
          new OauthDependencyBuilder(ReadingsArenaConfig.DEP_NAME_OAUTH)
              .withPort(OauthDependencyBuilder.DEFAULT_OAUTH_PORT)
              .withListenIp("0.0.0.0")
              .withServerTlsPem(pem.certificatePem(), pem.privateKeyPem())
              .build();
      String schemaPath = ReadingsArenaConfig.findSchema("instrument_reading_db_schema.sql");
      String mssqlSchemaPath = ReadingsArenaConfig.findSchema("validation_db_schema.sql");
      List<String> pgSql =
          schemaPath.isEmpty() ? List.of() : List.of(Files.readString(Path.of(schemaPath)));
      List<String> mssqlSql =
          mssqlSchemaPath.isEmpty() ? List.of() : List.of(Files.readString(Path.of(mssqlSchemaPath)));
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
      String mssqlId = mssql.identifier();
      HttpDependency calibration =
          new HttpDependencyBuilder(ReadingsArenaConfig.DEP_NAME_CALIBRATION_HTTP)
              .withPort(ReadingsArenaConfig.CALIBRATION_HOST_PORT)
              .withContainerName(ReadingsArenaConfig.CALIBRATION_CONTAINER_NAME)
              .build();
      String calibrationId = calibration.identifier();
      ManagedHttpPlaybook calibrationPlaybook =
          new ManagedHttpPlaybookBuilder(ReadingsArenaConfig.PLAYBOOK_CALIBRATION_DEFAULT, calibration.identifier())
              .withMapping(
                  "POST",
                  ReadingsArenaConfig.CALIBRATION_VALIDATE_PATH,
                  200,
                  java.util.Map.of("valid", true))
              .build();
      String bin = ReadingsArenaConfig.findAxumBinary();
      boolean bazel = System.getenv("RUNFILES_DIR") != null;
      ExecutableComponentBuilder execB =
          new ExecutableComponentBuilder(ReadingsArenaConfig.COMPONENT_NAME_EXECUTABLE)
              .withExecutablePath(bin)
              .withEnvVar("RUST_LOG", "info")
              .withEnvVar("OAUTH_TLS_CA_PEM", oauthCa)
              .withRuntimeArg("web_app_port", String.valueOf(ReadingsArenaConfig.EXEC_WEB_APP_PORT))
              .withRuntimeArg("postgres_connection_string", ReadingsArenaConfig.postgresConnectionLocal())
              .withRuntimeArg("kafka_bootstrap", "localhost:" + ReadingsArenaConfig.KAFKA_PORT)
              .withRuntimeArg(
                  "calibration_url", "http://127.0.0.1:" + ReadingsArenaConfig.CALIBRATION_HOST_PORT)
              .withRuntimeArg("mssql_connection_string", ReadingsArenaConfig.mssqlConnectionLocal())
              .withRuntimeArg("oauth_issuer_url", OauthDependencyBuilder.OAUTH_ISSUER)
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
              .registerPlaybook(calibrationPlaybook, true);
      boolean containerized = false;
      if (OauthIssuerHosts.oauthIssuerHostIsNonLoopback()) {
        Path ctxPath = ReadingsArenaConfig.prepareDockerImageContext(bin);
        if (ctxPath != null) {
          var container =
              new ContainerizedComponentBuilder(ReadingsArenaConfig.COMPONENT_NAME_CONTAINERIZED, "Containerfile")
                  .withBuildContext(ctxPath.toString())
                  .withImageTag(ReadingsArenaConfig.DOCKER_IMAGE_TAG)
                  .withNetwork(ReadingsArenaConfig.NETWORK_NAME)
                  .withPortMapping(ReadingsArenaConfig.DOCKER_WEB_HOST_PORT, 3000)
                  .withHostMapping("host.docker.internal:host-gateway")
                  .withEnvVar("RUST_LOG", "info")
                  .withEnvVar("OAUTH_TLS_CA_PEM", oauthCa)
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
                  .withRuntimeArg("oauth_issuer_url", OauthDependencyBuilder.OAUTH_ISSUER)
                  .withReadinessCheck(
                      HttpReadinessCheck.create(),
                      "http://127.0.0.1:" + ReadingsArenaConfig.DOCKER_WEB_HOST_PORT + "/health")
                  .build();
          mb = mb.addComponent(container);
          containerized = true;
        }
      }
      Match match = mb.build();
      ClosedArena closed = new ClosedArena(ReadingsArenaConfig.CLOSED_ARENA_NAME, List.of(match));
      OpenArena openArena = closed.open();
      Session s =
          new Session(openArena, oauthCa, mssqlId, calibrationId, containerized);
      s.accessTokenCache =
          ReadingsArenaConfig.fetchAccessToken(s.oauthCaPem, null);
      return s;
    }

    void close() {
      arena.close();
    }
  }
}
