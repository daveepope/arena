package dev.arena.examples.readings.springboot;

import java.util.HashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class ConnParse {

  private static final Pattern MSSQL_SRV =
      Pattern.compile("Server=tcp:([^,;]+),(\\d+)", Pattern.CASE_INSENSITIVE);
  private static final Pattern MSSQL_DB =
      Pattern.compile("Database=([^;]+)", Pattern.CASE_INSENSITIVE);
  private static final Pattern MSSQL_UID =
      Pattern.compile("User Id=([^;]+)", Pattern.CASE_INSENSITIVE);
  private static final Pattern MSSQL_PWD =
      Pattern.compile("Password=([^;]+)", Pattern.CASE_INSENSITIVE);

  public record PostgresConn(String jdbcUrl, String user, String password) {}

  public record MssqlConn(String jdbcUrl, String user, String password) {}

  private ConnParse() {}

  public static PostgresConn postgresConnFromLibpq(String libpq) throws Exception {
    Map<String, String> parts = new HashMap<>();
    for (String raw : libpq.split("\\s+")) {
      int eq = raw.indexOf('=');
      if (eq > 0) {
        parts.put(raw.substring(0, eq).trim(), raw.substring(eq + 1).trim());
      }
    }
    String user = parts.get("user");
    String password = parts.get("password");
    String host = parts.get("host");
    String port = parts.getOrDefault("port", "5432");
    String dbname = parts.get("dbname");
    if (user == null || password == null || host == null || dbname == null) {
      throw new IllegalArgumentException("postgres connection string incomplete");
    }
    String jdbcUrl = "jdbc:postgresql://" + host + ":" + port + "/" + dbname;
    return new PostgresConn(jdbcUrl, user, password);
  }

  public static MssqlConn mssqlConnFromAdo(String ado) {
    Matcher srv = MSSQL_SRV.matcher(ado);
    Matcher db = MSSQL_DB.matcher(ado);
    Matcher uid = MSSQL_UID.matcher(ado);
    Matcher pwd = MSSQL_PWD.matcher(ado);
    if (!srv.find() || !db.find() || !uid.find() || !pwd.find()) {
      throw new IllegalArgumentException("mssql connection string incomplete");
    }
    String host = srv.group(1).trim();
    String port = srv.group(2).trim();
    String database = db.group(1).trim();
    String user = uid.group(1).trim();
    String password = pwd.group(1).trim();
    String jdbcUrl =
        "jdbc:sqlserver://"
            + host
            + ":"
            + port
            + ";databaseName="
            + database
            + ";encrypt=true;trustServerCertificate=true";
    return new MssqlConn(jdbcUrl, user, password);
  }
}
