package arena.examples.readings.springboot;

import arena.examples.readings.springboot.workflow.DeviceState;
import com.fasterxml.jackson.annotation.JsonProperty;

public record DeviceStateResponse(
    @JsonProperty("device_id") long deviceId,
    DeviceState state,
    @JsonProperty("transition_count") int transitionCount) {}
