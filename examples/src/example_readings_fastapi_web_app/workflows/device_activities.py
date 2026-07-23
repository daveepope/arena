from temporalio import activity


@activity.defn
async def power_on(device_id: int) -> str:
    activity.logger.info("device %s powered on", device_id)
    return f"device {device_id} powered on"


@activity.defn
async def power_off(device_id: int) -> str:
    activity.logger.info("device %s powered off", device_id)
    return f"device {device_id} powered off"


@activity.defn
async def enter_error(device_id: int) -> str:
    activity.logger.info("device %s entered error state", device_id)
    return f"device {device_id} entered error state"
