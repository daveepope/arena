import asyncio

from fastmssql import Connection, PoolConfig, SslConfig

from arena_pytest import (
    ActivePlaybook,
    HttpPlaybookBuilder,
    ManagedHttpPlaybook,
    ManagedLocalstackPlaybook,
    ManagedMssqlPlaybook,
    ManagedOraclePlaybook,
    ManagedPostgresPlaybook,
    UnmanagedPlaybook,
    ok_json,
    server_error,
    status,
)

from arena_config import (
    CALIBRATION_VALIDATE_PATH,
    PLAYBOOK_CALIBRATION_API_ERROR_PATH,
    PLAYBOOK_CALIBRATION_API_HAPPY_PATH,
    PLAYBOOK_CALIBRATION_API_FLAKY_PATH,
    PLAYBOOK_EVENTS_PURGE,
    PLAYBOOK_VALIDATION_DB_SCOPED,
    PLAYBOOK_WEATHER_DB_SCOPED,
)


class CalibrationApiHappyPathPlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_CALIBRATION_API_HAPPY_PATH,
            dependency_identifier=dependency_identifier,
            builder=(
                HttpPlaybookBuilder(dependency_identifier)
                .post(CALIBRATION_VALIDATE_PATH)
                .will_return(ok_json({"valid": True}))
            ),
        )


class CalibrationApiErrorPathPlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_CALIBRATION_API_ERROR_PATH,
            dependency_identifier=dependency_identifier,
            builder=(
                HttpPlaybookBuilder(dependency_identifier)
                .post(CALIBRATION_VALIDATE_PATH)
                .will_return(server_error())
            ),
        )


class CalibrationApiFlakyPlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_CALIBRATION_API_FLAKY_PATH,
            dependency_identifier=dependency_identifier,
            builder=(
                HttpPlaybookBuilder(dependency_identifier)
                .post(CALIBRATION_VALIDATE_PATH)
                .will_return(server_error())
                .then_return(status(503))
                .then_return(ok_json({"valid": True}))
            ),
        )


class ResetValidationDbPlaybook(ManagedMssqlPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_VALIDATION_DB_SCOPED,
            dependency_identifier=dependency_identifier,
        )


class ResetWeatherDbPlaybook(ManagedOraclePlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_WEATHER_DB_SCOPED,
            dependency_identifier=dependency_identifier,
        )


class ResetReadingsDbPlaybook(ManagedPostgresPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier="test-readings-db-scoped",
            dependency_identifier=dependency_identifier,
        )


class EventsPurgePlaybook(ManagedLocalstackPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_EVENTS_PURGE,
            dependency_identifier=dependency_identifier,
        )


SEED_VALIDATION_READING_USER = "Seeded By Unmanaged Playbook"
SEED_VALIDATION_READING_VALUE = 42


async def connect_validation_db(connection_string: str) -> Connection:
    conn = Connection(
        connection_string=connection_string,
        ssl_config=SslConfig.development(),
        pool_config=PoolConfig.one(),
    )
    await conn.connect()
    return conn


class SeedValidationReadingPlaybook(UnmanagedPlaybook):
    def __init__(self, connection_string: str):
        self._connection_string = connection_string

    def identifier(self) -> str:
        return "seed-validation-reading"

    def run(self, arena) -> ActivePlaybook:
        asyncio.run(self._seed())
        return ActivePlaybook(None, 0)

    async def _seed(self) -> None:
        conn = await connect_validation_db(self._connection_string)
        try:
            await conn.execute(
                "INSERT INTO dbo.validation_results (user_name, value, valid) "
                "VALUES (@P1, @P2, @P3)",
                [SEED_VALIDATION_READING_USER, SEED_VALIDATION_READING_VALUE, 1],
            )
        finally:
            await conn.disconnect()
