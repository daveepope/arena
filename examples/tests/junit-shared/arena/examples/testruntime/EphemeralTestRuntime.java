package arena.examples.testruntime;

import arena.junit.ArenaHost;
import arena.junit.ffi.PortSearchStrategy;
import java.util.Map;
import java.util.UUID;

public final class EphemeralTestRuntime {

  public static final int PORT_SLOT_COUNT = 13;
  private static final int EPHEMERAL_PORT_RANGE_START = 20300;
  private static final int EPHEMERAL_PORT_RANGE_END = 20600;

  private static final Map<String, int[]> TARGET_PORT_RANGES =
      Map.of(
          "//examples:example-spring-boot-component-test", new int[] {20300, 20400},
          "//examples:example-spring-boot-chained-component-test", new int[] {20400, 20500},
          "//examples:example-axum-junit-component-test", new int[] {20500, 20600});

  private static final EphemeralTestRuntime INSTANCE = new EphemeralTestRuntime();

  public final String runSuffix;
  public final int execWebAppPort;
  public final int dockerWebHostPort;
  public final int kafkaPort;
  public final int calibrationHostPort;
  public final int postgresPort;
  public final int mssqlPort;
  public final int oraclePort;
  public final int oauthPort;
  public final int localstackHostPort;
  public final int temporalGrpcPort;
  public final int temporalUiPort;
  public final int smtpPort;
  public final int smtpUiPort;
  public final String oauthIssuer;

  private EphemeralTestRuntime() {
    runSuffix = UUID.randomUUID().toString().replace("-", "");
    int[] ports = allocateDistinctTcpPorts(PORT_SLOT_COUNT);
    execWebAppPort = ports[0];
    dockerWebHostPort = ports[1];
    kafkaPort = ports[2];
    calibrationHostPort = ports[3];
    postgresPort = ports[4];
    mssqlPort = ports[5];
    oraclePort = ports[6];
    oauthPort = ports[7];
    localstackHostPort = ports[8];
    temporalGrpcPort = ports[9];
    temporalUiPort = ports[10];
    smtpPort = ports[11];
    smtpUiPort = ports[12];
    oauthIssuer = "https://127.0.0.1:" + oauthPort;
  }

  public static EphemeralTestRuntime get() {
    return INSTANCE;
  }

  public String networkName(String base) {
    return namespaced(base);
  }

  public String containerName(String base) {
    return namespaced(base);
  }

  public String namespaced(String base) {
    return base + "-" + runSuffix;
  }

  public static int ephemeralTcpPort() {
    return allocateDistinctTcpPorts(1)[0];
  }

  static int[] portRangeForTarget(String target) {
    int[] range = target == null ? null : TARGET_PORT_RANGES.get(target);
    return range == null ? new int[] {EPHEMERAL_PORT_RANGE_START, EPHEMERAL_PORT_RANGE_END} : range;
  }

  private static int[] allocateDistinctTcpPorts(int count) {
    int[] range = portRangeForTarget(System.getenv("TEST_TARGET"));
    int[] ports = new int[count];
    for (int i = 0; i < count; i++) {
      ports[i] = ArenaHost.findAvailablePort(range[0], range[1], PortSearchStrategy.RANDOM);
    }
    return ports;
  }
}
