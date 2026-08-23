package bench;

import arena.junit.ClosedArena;
import arena.junit.OpenArena;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.dep.PostgresDependency;
import arena.junit.dep.PostgresDependencyBuilder;
import arena.junit.ffi.ArenaLogLevel;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.playbook.ActivePlaybook;
import arena.junit.playbook.ActivePostgresPlaybook;
import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.playbook.ManagedPostgresPlaybook;
import arena.junit.playbook.Playbook;
import arena.junit.playbook.UnmanagedPlaybook;

import com.sun.jna.Pointer;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.security.SecureRandom;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.util.ArrayList;
import java.util.List;

public final class Bench {

  private static final SecureRandom RANDOM = new SecureRandom();

  private static final String BENCHMARK_TABLE_SQL =
      "CREATE TABLE benchmark ("
          + "id SERIAL PRIMARY KEY, "
          + "version TEXT NOT NULL, "
          + "phase TEXT NOT NULL, "
          + "duration_ms DOUBLE PRECISION NOT NULL, "
          + "recorded_at TIMESTAMPTZ NOT NULL DEFAULT now())";

  private static String dbUrl;
  private static String dbUser;
  private static String dbPassword;
  private static String version;

  private static final class ManagedPostgresVerifyPlaybook extends ManagedPostgresPlaybook {
    ManagedPostgresVerifyPlaybook(String identifier, String dependencyIdentifier) {
      super(identifier, dependencyIdentifier);
    }
  }

  private static final class UnmanagedPostgresVerifyPlaybook implements Playbook, UnmanagedPlaybook {
    private final ManagedPostgresVerifyPlaybook managed;

    UnmanagedPostgresVerifyPlaybook(ManagedPostgresVerifyPlaybook managed) {
      this.managed = managed;
    }

    @Override
    public String identifier() {
      return "bench-postgres-unmanaged-verify";
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      ActivePostgresPlaybook active = managed.run(arena);
      active.verify("SELECT 1", 1);
      return active;
    }
  }

  private static final class UnmanagedHttpVerifyPlaybook implements Playbook, UnmanagedPlaybook {
    @Override
    public String identifier() {
      return "bench-http-unmanaged-verify";
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      try {
        HttpClient client = HttpClient.newHttpClient();
        HttpRequest request =
            HttpRequest.newBuilder(URI.create("http://127.0.0.1:18080/health")).GET().build();
        HttpResponse<Void> response = client.send(request, HttpResponse.BodyHandlers.discarding());
        if (response.statusCode() != 200) {
          throw new IllegalStateException("expected 200, got " + response.statusCode());
        }
      } catch (Exception e) {
        throw new IllegalStateException(e);
      }
      return new NoopActivePlaybook();
    }
  }

  private static final class NoopActivePlaybook extends ActivePlaybook {
    NoopActivePlaybook() {
      super(Pointer.NULL);
    }
  }

  public static void main(String[] args) throws Exception {
    version = args[0];
    int iterations = Integer.parseInt(args[1]);

    String suffix = randomToken(4);
    String dbName = "bench_" + suffix;
    dbUser = "bench_" + suffix;
    dbPassword = randomToken(12);
    dbUrl = "jdbc:postgresql://127.0.0.1:15432/" + dbName;

    PostgresDependency postgres =
        new PostgresDependencyBuilder("bench-postgres")
            .withPort(15432)
            .withDatabaseName(dbName)
            .withDatabaseUsername(dbUser)
            .withDatabasePassword(dbPassword)
            .withStartupSqlScripts(List.of(BENCHMARK_TABLE_SQL))
            .build();
    HttpDependency http = new HttpDependencyBuilder("bench-http").withPort(18080).build();

    ManagedPostgresVerifyPlaybook managedPostgres =
        new ManagedPostgresVerifyPlaybook("bench-postgres-managed", postgres.identifier());
    ManagedHttpPlaybook managedHttp =
        ManagedHttpPlaybook.fromBuilder(
            "bench-http-managed", http.identifier(), b -> b.get("/health").willReturn(200));

    Match match =
        new MatchBuilder("bench-match")
            .addDependency(postgres)
            .addDependency(http)
            .registerPlaybook(managedPostgres, true)
            .registerPlaybook(managedHttp, true)
            .build();
    ClosedArena closed = new ClosedArena("bench-arena", List.of(match), ArenaLogLevel.ERROR);

    long e2eStart = System.nanoTime();

    long openStart = System.nanoTime();
    OpenArena arena = closed.open();
    double openMs = (System.nanoTime() - openStart) / 1_000_000.0;

    double closeMs;
    double[] sorted;
    try {
      new UnmanagedPostgresVerifyPlaybook(managedPostgres).run(arena);
      new UnmanagedHttpVerifyPlaybook().run(arena);

      HttpClient client = HttpClient.newHttpClient();
      try (Connection conn = DriverManager.getConnection(dbUrl, dbUser, dbPassword)) {
        List<Double> iterationMs = new ArrayList<>();
        for (int n = 0; n < iterations; n++) {
          iterationMs.add(runIteration(n, arena, managedPostgres, client, conn));
        }
        sorted = iterationMs.stream().mapToDouble(Double::doubleValue).sorted().toArray();
      }
    } finally {
      long closeStart = System.nanoTime();
      arena.close();
      closeMs = (System.nanoTime() - closeStart) / 1_000_000.0;
    }

    double e2eMs = (System.nanoTime() - e2eStart) / 1_000_000.0;

    System.out.printf(
        "version=%s open_ms=%.2f iterations=%d interact_min_ms=%.2f interact_ms=%.2f "
            + "interact_p95_ms=%.2f interact_max_ms=%.2f close_ms=%.2f e2e_ms=%.2f%n",
        version, openMs, iterations, sorted[0], percentile(sorted, 0.5), percentile(sorted, 0.95),
        sorted[sorted.length - 1], closeMs, e2eMs);
  }

  private static double runIteration(
      int n, OpenArena arena, ManagedPostgresVerifyPlaybook managedPostgres,
      HttpClient client, Connection conn) throws Exception {
    long iterStart = System.nanoTime();

    HttpRequest request =
        HttpRequest.newBuilder(URI.create("http://127.0.0.1:18080/health")).GET().build();
    HttpResponse<Void> response = client.send(request, HttpResponse.BodyHandlers.discarding());
    if (response.statusCode() != 200) {
      throw new IllegalStateException("expected 200 from playbook, got " + response.statusCode());
    }
    double httpMs = (System.nanoTime() - iterStart) / 1_000_000.0;

    double readBackMs = recordAndReadBack(conn, "iter-" + n, httpMs);
    if (Math.abs(readBackMs - httpMs) > 1e-6) {
      throw new IllegalStateException("benchmark table read-back mismatch: wrote " + httpMs + " read " + readBackMs);
    }

    ActivePostgresPlaybook activePostgres = managedPostgres.run(arena);
    activePostgres.verify("SELECT 1", 1);

    return (System.nanoTime() - iterStart) / 1_000_000.0;
  }

  private static double recordAndReadBack(Connection conn, String phase, double durationMs) throws Exception {
    try (PreparedStatement insert =
        conn.prepareStatement(
            "INSERT INTO benchmark (version, phase, duration_ms) VALUES (?, ?, ?)")) {
      insert.setString(1, version);
      insert.setString(2, phase);
      insert.setDouble(3, durationMs);
      insert.executeUpdate();
    }
    try (PreparedStatement select =
        conn.prepareStatement(
            "SELECT duration_ms FROM benchmark WHERE version = ? AND phase = ? "
                + "ORDER BY id DESC LIMIT 1")) {
      select.setString(1, version);
      select.setString(2, phase);
      try (ResultSet rs = select.executeQuery()) {
        rs.next();
        return rs.getDouble(1);
      }
    }
  }

  private static double percentile(double[] sorted, double pct) {
    int idx = (int) Math.round(pct * (sorted.length - 1));
    return sorted[Math.min(sorted.length - 1, idx)];
  }

  private static String randomToken(int numBytes) {
    byte[] bytes = new byte[numBytes];
    RANDOM.nextBytes(bytes);
    StringBuilder sb = new StringBuilder();
    for (byte b : bytes) {
      sb.append(String.format("%02x", b));
    }
    return sb.toString();
  }
}
