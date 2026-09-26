import asyncio
from dataclasses import dataclass
from datetime import timedelta
from enum import StrEnum
from typing import Optional

from temporalio import workflow
from temporalio.exceptions import ApplicationError

from example_readings_fastapi_web_app.workflows.device_activities import (
    enter_error,
    power_off,
    power_on,
)

ACTIVITY_TIMEOUT = timedelta(seconds=10)
TRANSITION_TIMEOUT = timedelta(seconds=30)


class DeviceState(StrEnum):
    ON = "ON"
    OFF = "OFF"
    ERROR = "ERROR"


@dataclass
class DeviceSnapshot:
    state: DeviceState
    transition_count: int


@workflow.defn
class DeviceLifecycleWorkflow:
    def __init__(self) -> None:
        self._state = DeviceState.OFF
        self._requested: Optional[DeviceState] = None
        self._stop_requested = False
        self._transition_count = 0

    @workflow.run
    async def run(self, device_id: int) -> None:
        while not self._stop_requested:
            await workflow.wait_condition(
                lambda: self._requested is not None or self._stop_requested
            )
            if self._stop_requested:
                break
            target = self._requested
            self._requested = None
            self._state = await self._apply_transition(device_id, target)
            self._transition_count += 1

    @workflow.update
    async def request_transition(self, target: DeviceState) -> DeviceSnapshot:
        await workflow.wait_condition(
            lambda: self._requested is None or self._stop_requested
        )
        if self._stop_requested:
            raise ApplicationError("device is stopping", non_retryable=True)
        count_before = self._transition_count
        self._requested = target
        try:
            await workflow.wait_condition(
                lambda: self._transition_count != count_before,
                timeout=TRANSITION_TIMEOUT,
            )
        except asyncio.TimeoutError as exc:
            raise ApplicationError(
                f"device transition to {target} was not applied within"
                f" {TRANSITION_TIMEOUT}",
                non_retryable=True,
            ) from exc
        return self.snapshot()

    @workflow.signal
    def stop(self) -> None:
        self._stop_requested = True

    @workflow.query
    def snapshot(self) -> DeviceSnapshot:
        return DeviceSnapshot(
            state=self._state, transition_count=self._transition_count
        )

    async def _apply_transition(self, device_id: int, target: DeviceState) -> DeviceState:
        if target == DeviceState.ON:
            await workflow.execute_activity(
                power_on, device_id, start_to_close_timeout=ACTIVITY_TIMEOUT
            )
            return DeviceState.ON
        if target == DeviceState.OFF:
            await workflow.execute_activity(
                power_off, device_id, start_to_close_timeout=ACTIVITY_TIMEOUT
            )
            return DeviceState.OFF
        if target == DeviceState.ERROR:
            await workflow.execute_activity(
                enter_error, device_id, start_to_close_timeout=ACTIVITY_TIMEOUT
            )
            return DeviceState.ERROR
        raise ValueError(f"unknown device state: {target}")
