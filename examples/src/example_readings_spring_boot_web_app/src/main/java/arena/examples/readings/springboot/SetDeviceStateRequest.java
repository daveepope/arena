package arena.examples.readings.springboot;

import arena.examples.readings.springboot.workflow.DeviceState;
import jakarta.validation.constraints.NotNull;

public record SetDeviceStateRequest(@NotNull DeviceState target) {}
