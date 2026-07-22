package arena.examples.readings.springboot.workflow;

import io.temporal.activity.ActivityInterface;
import io.temporal.activity.ActivityMethod;

@ActivityInterface
public interface DeviceActivities {

  @ActivityMethod
  String powerOn(long deviceId);

  @ActivityMethod
  String powerOff(long deviceId);

  @ActivityMethod
  String enterError(long deviceId);
}
