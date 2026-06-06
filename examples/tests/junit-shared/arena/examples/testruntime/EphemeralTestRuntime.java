package arena.examples.testruntime;

import java.io.IOException;
import java.net.ServerSocket;
import java.util.UUID;

public final class EphemeralTestRuntime {

  private static final EphemeralTestRuntime INSTANCE = new EphemeralTestRuntime();

  public final String runSuffix;
  public final int execWebAppPort;
  public final int dockerWebHostPort;
  public final int kafkaPort;
  public final int calibrationHostPort;
  public final int postgresPort;
  public final int mssqlPort;
  public final int oauthPort;
  public final int localstackHostPort;
  public final String oauthIssuer;

  private EphemeralTestRuntime() {
    runSuffix = UUID.randomUUID().toString().replace("-", "");
    execWebAppPort = ephemeralTcpPort();
    dockerWebHostPort = ephemeralTcpPort();
    kafkaPort = ephemeralTcpPort();
    calibrationHostPort = ephemeralTcpPort();
    postgresPort = ephemeralTcpPort();
    mssqlPort = ephemeralTcpPort();
    oauthPort = ephemeralTcpPort();
    localstackHostPort = ephemeralTcpPort();
    oauthIssuer = "https://127.0.0.1:" + oauthPort;
  }

  public static EphemeralTestRuntime get() {
    return INSTANCE;
  }

  public String networkName(String base) {
    return base + "-" + runSuffix;
  }

  public String containerName(String base) {
    return base + "-" + runSuffix;
  }

  public static int ephemeralTcpPort() {
    try (ServerSocket socket = new ServerSocket(0)) {
      return socket.getLocalPort();
    } catch (IOException e) {
      throw new ExceptionInInitializerError(e);
    }
  }
}
