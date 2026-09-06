import asyncio
from contextlib import asynccontextmanager
from typing import List

from fastapi import APIRouter, HTTPException, Request
from pydantic import BaseModel
from temporalio.client import WorkflowHandle
from temporalio.service import RPCError, RPCStatusCode

from example_readings_fastapi_web_app.mail import send_device_provisioned_email
from example_readings_fastapi_web_app.workflows.device_workflow import (
    DeviceLifecycleWorkflow,
    DeviceSnapshot,
    DeviceState,
)

router = APIRouter(prefix="/devices", tags=["devices"])

class CreateDeviceBody(BaseModel):
    name: str


class CreateDeviceResponse(BaseModel):
    id: int
    name: str


class DeviceRow(BaseModel):
    id: int
    name: str


class SetDeviceStateBody(BaseModel):
    target: DeviceState


class DeviceStateResponse(BaseModel):
    device_id: int
    state: DeviceState
    transition_count: int


def _workflow_id(device_id: int) -> str:
    return f"device-{device_id}"


def _workflow_handle(request: Request, device_id: int) -> WorkflowHandle:
    client = request.app.state.temporal_client
    return client.get_workflow_handle(_workflow_id(device_id))


@asynccontextmanager
async def _translate_not_found(device_id: int):
    try:
        yield
    except RPCError as exc:
        if exc.status == RPCStatusCode.NOT_FOUND:
            raise HTTPException(
                status_code=404, detail=f"device not found: {device_id}"
            ) from exc
        raise


@router.get("")
async def list_devices(request: Request) -> List[DeviceRow]:
    pool = request.app.state.pool
    async with pool.acquire() as conn:
        rows = await conn.fetch(
            "select id, name from instrument_reading.device order by id"
        )
    return [DeviceRow(id=r["id"], name=r["name"]) for r in rows]


@router.post("")
async def create_device(request: Request, body: CreateDeviceBody) -> CreateDeviceResponse:
    st = request.app.state
    pool = st.pool
    async with pool.acquire() as conn:
        device_id = await conn.fetchval(
            "insert into instrument_reading.device(name) values ($1) returning id",
            body.name,
        )
    try:
        await st.temporal_client.start_workflow(
            DeviceLifecycleWorkflow.run,
            device_id,
            id=_workflow_id(device_id),
            task_queue=st.temporal_task_queue,
        )
    except Exception as exc:
        async with pool.acquire() as conn:
            await conn.execute(
                "delete from instrument_reading.device where id = $1", device_id
            )
        raise HTTPException(
            status_code=502,
            detail=f"failed to start device workflow for device {device_id}",
        ) from exc
    try:
        await asyncio.to_thread(
            send_device_provisioned_email,
            st.smtp_host,
            st.smtp_port,
            device_id,
            body.name,
        )
    except Exception:
        pass
    return CreateDeviceResponse(id=device_id, name=body.name)


@router.get("/{device_id}/state")
async def get_device_state(request: Request, device_id: int) -> DeviceStateResponse:
    handle = _workflow_handle(request, device_id)
    async with _translate_not_found(device_id):
        snapshot = await handle.query(DeviceLifecycleWorkflow.snapshot)
    return DeviceStateResponse(
        device_id=device_id,
        state=snapshot.state,
        transition_count=snapshot.transition_count,
    )


@router.post("/{device_id}/state")
async def set_device_state(
    request: Request, device_id: int, body: SetDeviceStateBody
) -> DeviceStateResponse:
    handle = _workflow_handle(request, device_id)
    async with _translate_not_found(device_id):
        snapshot = await handle.execute_update(
            DeviceLifecycleWorkflow.request_transition, body.target
        )
    return DeviceStateResponse(
        device_id=device_id,
        state=snapshot.state,
        transition_count=snapshot.transition_count,
    )


@router.delete("/{device_id}")
async def stop_device(request: Request, device_id: int) -> None:
    handle = _workflow_handle(request, device_id)
    async with _translate_not_found(device_id):
        await handle.signal(DeviceLifecycleWorkflow.stop)
