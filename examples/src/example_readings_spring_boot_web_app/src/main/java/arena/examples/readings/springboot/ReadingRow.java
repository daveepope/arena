package arena.examples.readings.springboot;

import com.fasterxml.jackson.annotation.JsonProperty;

public record ReadingRow(
    long id, @JsonProperty("user_name") String userName, int value, String comment) {}
