import smtplib
from email.message import EmailMessage


def send_device_provisioned_email(
    smtp_host: str,
    smtp_port: int,
    device_id: int,
    device_name: str,
) -> None:
    message = EmailMessage()
    message["Subject"] = f"Device provisioned: {device_name} ({device_id})"
    message["From"] = "no-reply@arena.example"
    message["To"] = "operations@arena.example"
    message.set_content(
        f"Device {device_name} (id={device_id}) has been provisioned."
    )
    with smtplib.SMTP(smtp_host, smtp_port, timeout=10) as client:
        client.send_message(message)
