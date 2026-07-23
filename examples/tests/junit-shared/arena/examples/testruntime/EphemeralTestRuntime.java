package arena.examples.testruntime;

import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.util.UUID;

public final class EphemeralTestRuntime {

  public static final int PORT_SLOT_COUNT = 10;

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
  public final int temporalGrpcPort;
  public final int temporalUiPort;
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
    oauthPort = ports[6];
    localstackHostPort = ports[7];
    temporalGrpcPort = ports[8];
    temporalUiPort = ports[9];
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

  private static int[] allocateDistinctTcpPorts(int count) {
    ServerSocket[] sockets = new ServerSocket[count];
    try {
      for (int i = 0; i < count; i++) {
        ServerSocket socket = new ServerSocket();
        socket.bind(new InetSocketAddress("127.0.0.1", 0));
        sockets[i] = socket;
      }
      int[] ports = new int[count];
      for (int i = 0; i < count; i++) {
        ports[i] = sockets[i].getLocalPort();
      }
      return ports;
    } catch (IOException e) {
      throw new ExceptionInInitializerError(e);
    } finally {
      for (ServerSocket socket : sockets) {
        if (socket != null) {
          try {
            socket.close();
          } catch (IOException ignored) {
          }
        }
      }
    }
  }
}
