package arena.examples.playbooks;

import arena.junit.OpenArena;
import arena.junit.playbook.ActivePlaybook;
import arena.junit.playbook.Playbook;
import arena.junit.playbook.UnmanagedPlaybook;
import com.sun.jna.Pointer;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;

public final class SeedValidationReadingPlaybook implements Playbook, UnmanagedPlaybook {
  public static final String SEED_USER_NAME = "Seeded By Unmanaged Playbook";
  public static final int SEED_VALUE = 42;

  private final String jdbcUrl;

  public SeedValidationReadingPlaybook(String jdbcUrl) {
    this.jdbcUrl = jdbcUrl;
  }

  public static String jdbcUrl(int port, String databaseName, String user, String password) {
    return "jdbc:sqlserver://localhost:"
        + port
        + ";databaseName="
        + databaseName
        + ";user="
        + user
        + ";password="
        + password
        + ";trustServerCertificate=true;encrypt=false;";
  }

  public static int countSeededRows(String jdbcUrl) throws Exception {
    try (Connection connection = DriverManager.getConnection(jdbcUrl);
        PreparedStatement statement =
            connection.prepareStatement(
                "SELECT COUNT(*) FROM dbo.validation_results WHERE user_name = ? AND value = ?")) {
      statement.setString(1, SEED_USER_NAME);
      statement.setInt(2, SEED_VALUE);
      try (ResultSet resultSet = statement.executeQuery()) {
        resultSet.next();
        return resultSet.getInt(1);
      }
    }
  }

  @Override
  public String identifier() {
    return "seed-validation-reading";
  }

  @Override
  public ActivePlaybook run(OpenArena arena) {
    try (Connection connection = DriverManager.getConnection(jdbcUrl);
        PreparedStatement statement =
            connection.prepareStatement(
                "INSERT INTO dbo.validation_results (user_name, value, valid) VALUES (?, ?, ?)")) {
      statement.setString(1, SEED_USER_NAME);
      statement.setInt(2, SEED_VALUE);
      statement.setBoolean(3, true);
      statement.executeUpdate();
    } catch (Exception e) {
      throw new IllegalStateException("failed to seed dbo.validation_results row", e);
    }
    return new NoopActivePlaybook();
  }

  private static final class NoopActivePlaybook extends ActivePlaybook {
    NoopActivePlaybook() {
      super(Pointer.NULL);
    }
  }
}
