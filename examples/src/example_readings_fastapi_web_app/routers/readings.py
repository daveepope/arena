import asyncio
from typing import Optional

from fastapi import APIRouter, HTTPException, Request
from pydantic import BaseModel

from example_readings_fastapi_web_app.event_publish import put_reading_created_event

router = APIRouter(prefix="/readings", tags=["readings"])


class CreateReadingBody(BaseModel):
    user_name: str
    value: int
    comment: Optional[str] = None


class CreateReadingResponse(BaseModel):
    valid: bool
    id: Optional[int] = None


@router.get("")
async def list_readings(request: Request):
    pool = request.app.state.pool
    async with pool.acquire() as conn:
        rows = await conn.fetch(
            """
            select r.id, u.name as user_name, r.value, r.comment
            from instrument_reading.reading r
            join instrument_reading."user" u on u.id = r."userId"
            order by r.id desc
            limit 50
            """
        )
    return [dict(r) for r in rows]


@router.post("")
async def create_reading(request: Request, body: CreateReadingBody):
    st = request.app.state
    settings = st.settings
    try:
        r = await st.http.post(
            f"{settings.calibration_url.rstrip('/')}/api/v1/validate",
            json={"value": body.value},
        )
        r.raise_for_status()
        cal = r.json()
        valid = bool(cal.get("valid"))
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc
    try:
        await st.mssql.execute(
            "INSERT INTO dbo.validation_results (user_name, value, valid) VALUES (@P1, @P2, @P3)",
            [body.user_name, body.value, 1 if valid else 0],
        )
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc
    pool = st.pool
    async with pool.acquire() as conn:
        uid = await conn.fetchval(
            'select id from instrument_reading."user" where name = $1',
            body.user_name,
        )
        if uid is None:
            uid = await conn.fetchval(
                'insert into instrument_reading."user"(name) values ($1) returning id',
                body.user_name,
            )
        rid = await conn.fetchval(
            """
            insert into instrument_reading.reading("userId", value, comment)
            values ($1, $2, $3)
            returning id
            """,
            uid,
            body.value,
            body.comment,
        )
    try:
        await asyncio.to_thread(
            put_reading_created_event,
            st.events_client,
            settings.event_bus_name,
            settings.event_source,
            settings.reading_created_detail_type,
            int(rid),
            body.user_name,
            body.value,
            body.comment,
        )
    except Exception:
        pass
    return CreateReadingResponse(valid=valid, id=int(rid))
