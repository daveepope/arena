package arena.examples.readings.springboot.workflow;

import io.temporal.activity.ActivityOptions;
import io.temporal.workflow.Workflow;
import java.time.Duration;

public final class DeviceLifecycleWorkflowImpl implements DeviceLifecycleWorkflow {

  private static final ActivityOptions ACTIVITY_OPTIONS =
      ActivityOptions.newBuilder().setStartToCloseTimeout(Duration.ofSeconds(10)).build();

  private final DeviceActivities activities =
      Workflow.newActivityStub(DeviceActivities.class, ACTIVITY_OPTIONS);

  private DeviceState state = DeviceState.OFF;
  private DeviceState requested;
  private boolean stopRequested;
  private int transitionCount;

  @Override
  public void run(long deviceId) {
    while (!stopRequested) {
      Workflow.await(() -> requested != null || stopRequested);
      if (stopRequested) {
        break;
      }
      DeviceState target = requested;
      requested = null;
      state = applyTransition(deviceId, target);
      transitionCount++;
    }
  }

  @Override
  public void requestTransition(DeviceState target) {
    this.requested = target;
  }

  @Override
  public void stop() {
    this.stopRequested = true;
  }

  @Override
  public DeviceSnapshot snapshot() {
    return new DeviceSnapshot(state, transitionCount);
  }

  private DeviceState applyTransition(long deviceId, DeviceState target) {
    switch (target) {
      case ON:
        activities.powerOn(deviceId);
        return DeviceState.ON;
      case OFF:
        activities.powerOff(deviceId);
        return DeviceState.OFF;
      case ERROR:
        activities.enterError(deviceId);
        return DeviceState.ERROR;
      default:
        throw new IllegalArgumentException("unknown device state: " + target);
    }
  }
}
