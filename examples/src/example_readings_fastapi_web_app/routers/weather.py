from datetime import datetime, timezone
from typing import List

from fastapi import APIRouter, Request
from pydantic import BaseModel

router = APIRouter(prefix="/weather", tags=["weather"])


class CreateWeatherReportBody(BaseModel):
    precipitation: float
    humidity: float
    pressure: float


class CreateWeatherReportResponse(BaseModel):
    id: int


class WeatherReportResponse(BaseModel):
    id: int
    recorded_at: datetime
    precipitation: float
    humidity: float
    pressure: float


@router.post("")
async def create_weather_report(
    request: Request, body: CreateWeatherReportBody
) -> CreateWeatherReportResponse:
    pool = request.app.state.oracle_pool
    recorded_at = datetime.now(timezone.utc)
    async with pool.acquire() as conn:
        cursor = conn.cursor()
        id_var = cursor.var(int)
        await cursor.execute(
            """
            insert into weather_report (recorded_at, precipitation, humidity, pressure)
            values (:recorded_at, :precipitation, :humidity, :pressure)
            returning id into :id
            """,
            {
                "recorded_at": recorded_at,
                "precipitation": body.precipitation,
                "humidity": body.humidity,
                "pressure": body.pressure,
                "id": id_var,
            },
        )
        await conn.commit()
        report_id = int(id_var.getvalue()[0])
    return CreateWeatherReportResponse(id=report_id)


@router.get("")
async def list_weather_reports(request: Request) -> List[WeatherReportResponse]:
    pool = request.app.state.oracle_pool
    async with pool.acquire() as conn:
        cursor = conn.cursor()
        await cursor.execute(
            """
            select id, recorded_at, precipitation, humidity, pressure
            from weather_report
            order by id desc
            fetch first 50 rows only
            """
        )
        columns = [c[0].lower() for c in cursor.description]
        rows = await cursor.fetchall()
    return [WeatherReportResponse(**dict(zip(columns, row))) for row in rows]
