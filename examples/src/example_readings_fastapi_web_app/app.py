from contextlib import asynccontextmanager
import ssl

import asyncpg
import boto3
import httpx
import jwt
from fastapi import FastAPI, Request
from fastmssql import Connection, PoolConfig, SslConfig
from jwt import PyJWKClient
from starlette.middleware.base import BaseHTTPMiddleware
from starlette.responses import JSONResponse

from example_readings_fastapi_web_app.conn_parse import (
    asyncpg_dsn_from_libpq,
    mssql_fastmssql_connection_string,
)
from example_readings_fastapi_web_app.routers import health, readings
from example_readings_fastapi_web_app.settings import Settings


def build_ssl_context_ca_pem(pem: str) -> ssl.SSLContext:
    ctx = ssl.create_default_context()
    ctx.load_verify_locations(cadata=pem.encode())
    return ctx


def build_ssl_context_ca_file(path: str) -> ssl.SSLContext:
    ctx = ssl.create_default_context()
    ctx.load_verify_locations(cafile=path)
    return ctx


class BearerGateMiddleware(BaseHTTPMiddleware):
    async def dispatch(self, request: Request, call_next):
        if request.url.path == "/health":
            return await call_next(request)
        st = request.app.state
        auth = request.headers.get("Authorization")
        if not auth or not auth.lower().startswith("bearer "):
            return JSONResponse(status_code=401, content={"detail": "missing bearer"})
        token = auth.split(" ", 1)[1].strip()
        try:
            signing_key = st.jwk_client.get_signing_key_from_jwt(token)
            payload = jwt.decode(
                token,
                signing_key.key,
                algorithms=["RS256"],
                issuer=st.oauth_issuer,
                options={"verify_aud": False},
            )
        except Exception:
            return JSONResponse(status_code=401, content={"detail": "invalid token"})
        req_scopes = st.required_scopes
        if req_scopes:
            sc = payload.get("scope")
            if not isinstance(sc, str) or not sc.strip():
                return JSONResponse(status_code=401, content={"detail": "missing scope"})
            granted = set(sc.split())
            if not all(s in granted for s in req_scopes):
                return JSONResponse(status_code=401, content={"detail": "insufficient scope"})
        request.state.token_claims = payload
        return await call_next(request)


@asynccontextmanager
async def lifespan(app: FastAPI):
    settings = Settings()
    dsn = asyncpg_dsn_from_libpq(settings.postgres_connection_string)
    pool = await asyncpg.create_pool(dsn, min_size=1, max_size=5)
    mssql_cs = mssql_fastmssql_connection_string(settings.mssql_connection_string)
    mssql = Connection(
        connection_string=mssql_cs,
        ssl_config=SslConfig.development(),
        pool_config=PoolConfig.adaptive(8),
    )
    await mssql.connect()
    ca_path = settings.oauth_tls_ca_file.strip()
    if ca_path:
        ssl_ctx = build_ssl_context_ca_file(ca_path)
    else:
        ssl_ctx = build_ssl_context_ca_pem(settings.oauth_tls_ca_pem)
    issuer = settings.oauth_issuer_url.rstrip("/")
    jwks_url = f"{issuer}/.well-known/jwks.json"
    jwk_client = PyJWKClient(jwks_url, ssl_context=ssl_ctx)
    ev_kw: dict = {}
    if settings.aws_endpoint_url.strip():
        ev_kw["endpoint_url"] = settings.aws_endpoint_url.strip()
    events_client = boto3.client(
        "events",
        region_name=settings.aws_default_region,
        aws_access_key_id=settings.aws_access_key_id,
        aws_secret_access_key=settings.aws_secret_access_key,
        **ev_kw,
    )
    http = httpx.AsyncClient(timeout=30.0)
    req_scopes = [s for s in settings.oauth_required_access_token_scopes.split() if s]
    app.state.pool = pool
    app.state.http = http
    app.state.mssql = mssql
    app.state.settings = settings
    app.state.jwk_client = jwk_client
    app.state.oauth_issuer = issuer
    app.state.required_scopes = req_scopes
    app.state.events_client = events_client
    yield
    await http.aclose()
    await pool.close()
    await mssql.disconnect()


def create_app() -> FastAPI:
    app = FastAPI(lifespan=lifespan)
    app.include_router(health.router)
    app.include_router(readings.router)
    app.add_middleware(BearerGateMiddleware)
    return app


app = create_app()
