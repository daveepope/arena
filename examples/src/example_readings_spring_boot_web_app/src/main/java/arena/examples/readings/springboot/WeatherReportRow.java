package arena.examples.readings.springboot;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.time.Instant;

public record WeatherReportRow(
    long id,
    @JsonProperty("recorded_at") Instant recordedAt,
    double precipitation,
    double humidity,
    double pressure) {}
