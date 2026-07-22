package arena.examples.readings.springboot;

import jakarta.validation.constraints.NotBlank;

public record CreateDeviceRequest(@NotBlank String name) {}
