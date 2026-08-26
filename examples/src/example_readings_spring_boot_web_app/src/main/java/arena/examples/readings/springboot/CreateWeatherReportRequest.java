package arena.examples.readings.springboot;

import jakarta.validation.constraints.NotNull;

public record CreateWeatherReportRequest(
    @NotNull Double precipitation, @NotNull Double humidity, @NotNull Double pressure) {}
