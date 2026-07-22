package arena.examples.readings.springboot;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.springframework.beans.factory.annotation.Qualifier;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.http.HttpEntity;
import org.springframework.http.HttpHeaders;
import org.springframework.http.MediaType;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.stereotype.Service;
import org.springframework.web.client.RestTemplate;

@Service
public class ReadingsService {

  private static final ObjectMapper JSON = new ObjectMapper();

  private final JdbcTemplate pg;
  private final JdbcTemplate mssql;
  private final RestTemplate calibrationHttp;
  private final String calibrationBase;
  private final ReadingEventBridge events;

  public ReadingsService(
      @Qualifier("postgresJdbcTemplate") JdbcTemplate pg,
      @Qualifier("mssqlJdbcTemplate") JdbcTemplate mssql,
      @Value("${CALIBRATION_API_BASE_URL}") String calibrationUrl,
      ReadingEventBridge events) {
    this.pg = pg;
    this.mssql = mssql;
    this.calibrationHttp = new RestTemplate();
    this.calibrationBase = calibrationUrl.replaceAll("/+$", "");
    this.events = events;
  }

  public List<ReadingRow> listReadings() {
    return pg.query(
        """
        select r.id, u.name as user_name, r.value, r.comment
        from instrument_reading.reading r
        join instrument_reading."user" u on u.id = r."userId"
        order by r.id desc
        limit 50
        """,
        (rs, rowNum) ->
            new ReadingRow(
                rs.getLong("id"),
                rs.getString("user_name"),
                rs.getInt("value"),
                rs.getString("comment")));
  }

  public CreateReadingResponse createReading(CreateReadingRequest req) throws Exception {
    String validateUrl = calibrationBase + "/api/v1/validate";
    HttpHeaders headers = new HttpHeaders();
    headers.setContentType(MediaType.APPLICATION_JSON);
    Map<String, Object> calReq = new HashMap<>();
    calReq.put("value", req.value());
    HttpEntity<Map<String, Object>> entity = new HttpEntity<>(calReq, headers);
    String calRaw =
        calibrationHttp.postForObject(validateUrl, entity, String.class);
    JsonNode cal = JSON.readTree(calRaw);
    boolean valid = cal.path("valid").asBoolean(false);

    mssql.update(
        "INSERT INTO dbo.validation_results (user_name, value, valid) VALUES (?, ?, ?)",
        req.userName(),
        req.value(),
        valid);

    List<Long> existing =
        pg.query(
            "select id from instrument_reading.\"user\" where name = ?",
            (rs, rn) -> rs.getLong(1),
            req.userName());
    long userId =
        existing.isEmpty()
            ? pg.queryForObject(
                "insert into instrument_reading.\"user\"(name) values (?) returning id",
                Long.class,
                req.userName())
            : existing.get(0);

    long readingId =
        pg.queryForObject(
            """
            insert into instrument_reading.reading("userId", "deviceId", value, comment)
            values (?, ?, ?, ?)
            returning id
            """,
            Long.class,
            userId,
            req.deviceId(),
            req.value(),
            req.comment());

    try {
      events.publishReadingCreated(readingId, req.userName(), req.value(), req.comment());
    } catch (Exception ignored) {
    }

    return new CreateReadingResponse(valid, Math.toIntExact(readingId));
  }
}
