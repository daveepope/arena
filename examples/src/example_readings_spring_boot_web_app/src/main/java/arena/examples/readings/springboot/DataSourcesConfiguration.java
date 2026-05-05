package arena.examples.readings.springboot;

import javax.sql.DataSource;
import org.springframework.beans.factory.annotation.Qualifier;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.boot.jdbc.DataSourceBuilder;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.context.annotation.Primary;
import org.springframework.jdbc.core.JdbcTemplate;

@Configuration
public class DataSourcesConfiguration {

  @Bean("postgresDataSource")
  @Primary
  DataSource postgresDataSource(@Value("${POSTGRES_CONNECTION_STRING}") String libpq)
      throws Exception {
    ConnParse.PostgresConn c = ConnParse.postgresConnFromLibpq(libpq);
    return DataSourceBuilder.create()
        .url(c.jdbcUrl())
        .username(c.user())
        .password(c.password())
        .driverClassName("org.postgresql.Driver")
        .build();
  }

  @Bean("mssqlDataSource")
  DataSource mssqlDataSource(@Value("${MSSQL_CONNECTION_STRING}") String ado) {
    ConnParse.MssqlConn c = ConnParse.mssqlConnFromAdo(ado);
    return DataSourceBuilder.create()
        .url(c.jdbcUrl())
        .username(c.user())
        .password(c.password())
        .driverClassName("com.microsoft.sqlserver.jdbc.SQLServerDriver")
        .build();
  }

  @Bean("postgresJdbcTemplate")
  @Primary
  JdbcTemplate postgresJdbcTemplate(@Qualifier("postgresDataSource") DataSource ds) {
    return new JdbcTemplate(ds);
  }

  @Bean("mssqlJdbcTemplate")
  JdbcTemplate mssqlJdbcTemplate(@Qualifier("mssqlDataSource") DataSource ds) {
    return new JdbcTemplate(ds);
  }
}
