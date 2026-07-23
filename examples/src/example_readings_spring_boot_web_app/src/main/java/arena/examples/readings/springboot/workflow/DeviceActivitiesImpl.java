package arena.examples.readings.springboot.workflow;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class DeviceActivitiesImpl implements DeviceActivities {

  private static final Logger LOG = LoggerFactory.getLogger(DeviceActivitiesImpl.class);

  @Override
  public String powerOn(long deviceId) {
    LOG.info("device {} powered on", deviceId);
    return "device " + deviceId + " powered on";
  }

  @Override
  public String powerOff(long deviceId) {
    LOG.info("device {} powered off", deviceId);
    return "device " + deviceId + " powered off";
  }

  @Override
  public String enterError(long deviceId) {
    LOG.info("device {} entered error state", deviceId);
    return "device " + deviceId + " entered error state";
  }
}
