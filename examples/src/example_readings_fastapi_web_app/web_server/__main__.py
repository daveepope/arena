import os

import uvicorn


def main() -> None:
    port = int(os.environ["WEB_APP_PORT"])
    uvicorn.run(
        "example_readings_fastapi_web_app.app:app",
        host="0.0.0.0",
        port=port,
        factory=False,
        log_level="info",
    )


if __name__ == "__main__":
    main()
