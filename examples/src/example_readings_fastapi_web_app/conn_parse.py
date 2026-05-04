import re
import urllib.parse


def asyncpg_dsn_from_libpq(conn: str) -> str:
    parts: dict[str, str] = {}
    for raw in conn.split():
        if "=" in raw:
            k, _, v = raw.partition("=")
            parts[k.strip()] = v.strip()
    user = parts["user"]
    password = parts["password"]
    host = parts["host"]
    port = parts.get("port", "5432")
    dbname = parts["dbname"]
    u = urllib.parse.quote_plus(user)
    p = urllib.parse.quote_plus(password)
    return f"postgresql://{u}:{p}@{host}:{port}/{dbname}"


def mssql_fastmssql_connection_string(conn: str) -> str:
    srv = re.search(r"Server=tcp:([^,;]+),(\d+)", conn, re.I)
    db = re.search(r"Database=([^;]+)", conn, re.I)
    uid = re.search(r"User Id=([^;]+)", conn, re.I)
    pwd = re.search(r"Password=([^;]+)", conn, re.I)
    if not (srv and db and uid and pwd):
        raise ValueError("mssql connection string incomplete")
    host = srv.group(1).strip()
    port = srv.group(2).strip()
    database = db.group(1).strip()
    user = uid.group(1).strip()
    password = pwd.group(1).strip()
    return (
        f"Server=tcp:{host},{port};Database={database};User Id={user};Password={password};"
        "TrustServerCertificate=True;"
    )
