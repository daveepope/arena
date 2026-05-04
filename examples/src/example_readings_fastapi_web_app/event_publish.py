import json
from typing import Any


def put_reading_created_event(
    client: Any,
    event_bus_name: str,
    event_source: str,
    detail_type: str,
    reading_id: int,
    user_name: str,
    value: int,
    comment: str | None,
) -> None:
    detail = {
        "id": reading_id,
        "user_name": user_name,
        "value": value,
        "comment": comment,
    }
    client.put_events(
        Entries=[
            {
                "Source": event_source,
                "DetailType": detail_type,
                "Detail": json.dumps(detail),
                "EventBusName": event_bus_name,
            }
        ]
    )
