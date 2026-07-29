import smtplib
import ssl
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
    tls_context = ssl.create_default_context()
    tls_context.check_hostname = False
    tls_context.verify_mode = ssl.CERT_NONE
    with smtplib.SMTP(smtp_host, smtp_port, timeout=10) as client:
        client.starttls(context=tls_context)
        client.send_message(message)
