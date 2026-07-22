package arena.examples.readings.springboot;

import com.fasterxml.jackson.annotation.JsonProperty;
import jakarta.validation.constraints.NotBlank;
import jakarta.validation.constraints.NotNull;

public record CreateReadingRequest(
    @JsonProperty("user_name") @NotBlank String userName,
    int value,
    String comment,
    @JsonProperty("device_id") @NotNull Long deviceId) {}
