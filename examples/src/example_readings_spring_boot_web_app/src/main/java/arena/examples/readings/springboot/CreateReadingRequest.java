package arena.examples.readings.springboot;

import com.fasterxml.jackson.annotation.JsonProperty;
import jakarta.validation.constraints.NotBlank;

public record CreateReadingRequest(
    @JsonProperty("user_name") @NotBlank String userName, int value, String comment) {}
