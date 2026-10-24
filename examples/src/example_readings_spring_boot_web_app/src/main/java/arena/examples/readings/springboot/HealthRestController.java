package arena.examples.readings.springboot;

import org.springframework.beans.factory.annotation.Qualifier;
import org.springframework.http.HttpStatus;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
public class HealthRestController {

  private final JdbcTemplate postgres;
  private final JdbcTemplate mssql;
  private final JdbcTemplate oracle;

  HealthRestController(
      @Qualifier("postgresJdbcTemplate") JdbcTemplate postgres,
      @Qualifier("mssqlJdbcTemplate") JdbcTemplate mssql,
      @Qualifier("oracleJdbcTemplate") JdbcTemplate oracle) {
    this.postgres = postgres;
    this.mssql = mssql;
    this.oracle = oracle;
  }

  @GetMapping(value = "/health", produces = MediaType.TEXT_PLAIN_VALUE)
  public ResponseEntity<String> health() {
    try {
      postgres.queryForObject("SELECT 1", Integer.class);
      mssql.queryForObject("SELECT 1", Integer.class);
      oracle.queryForObject("SELECT 1 FROM dual", Integer.class);
    } catch (RuntimeException unreachable) {
      return ResponseEntity.status(HttpStatus.SERVICE_UNAVAILABLE).body("unavailable");
    }
    return ResponseEntity.ok("ok");
  }
}
