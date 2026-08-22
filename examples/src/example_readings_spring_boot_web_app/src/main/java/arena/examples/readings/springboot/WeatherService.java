package arena.examples.readings.springboot;

import java.sql.PreparedStatement;
import java.sql.Timestamp;
import java.time.Instant;
import java.util.List;
import org.springframework.beans.factory.annotation.Qualifier;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.jdbc.support.GeneratedKeyHolder;
import org.springframework.jdbc.support.KeyHolder;
import org.springframework.stereotype.Service;

@Service
public class WeatherService {

  private final JdbcTemplate oracle;

  public WeatherService(@Qualifier("oracleJdbcTemplate") JdbcTemplate oracle) {
    this.oracle = oracle;
  }

  public List<WeatherReportRow> listWeatherReports() {
    return oracle.query(
        """
        select id, recorded_at, precipitation, humidity, pressure
        from weather_report
        order by id desc
        fetch first 50 rows only
        """,
        (rs, rowNum) ->
            new WeatherReportRow(
                rs.getLong("id"),
                rs.getTimestamp("recorded_at").toInstant(),
                rs.getDouble("precipitation"),
                rs.getDouble("humidity"),
                rs.getDouble("pressure")));
  }

  public CreateWeatherReportResponse createWeatherReport(CreateWeatherReportRequest req) {
    Instant recordedAt = Instant.now();
    KeyHolder keyHolder = new GeneratedKeyHolder();
    oracle.update(
        connection -> {
          PreparedStatement ps =
              connection.prepareStatement(
                  """
                  insert into weather_report (recorded_at, precipitation, humidity, pressure)
                  values (?, ?, ?, ?)
                  """,
                  new String[] {"id"});
          ps.setTimestamp(1, Timestamp.from(recordedAt));
          ps.setDouble(2, req.precipitation());
          ps.setDouble(3, req.humidity());
          ps.setDouble(4, req.pressure());
          return ps;
        },
        keyHolder);
    return new CreateWeatherReportResponse(keyHolder.getKey().longValue());
  }
}
