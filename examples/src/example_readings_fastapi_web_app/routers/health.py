from fastapi import APIRouter
from starlette.responses import PlainTextResponse

router = APIRouter(tags=["health"])


@router.get("/health")
async def health():
    return PlainTextResponse("ok")
