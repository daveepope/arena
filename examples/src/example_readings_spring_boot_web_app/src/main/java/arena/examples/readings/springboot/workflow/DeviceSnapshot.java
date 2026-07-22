package arena.examples.readings.springboot.workflow;

public record DeviceSnapshot(DeviceState state, int transitionCount) {}
