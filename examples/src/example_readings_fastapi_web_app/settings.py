from pydantic import model_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(extra="ignore")

    postgres_connection_string: str
    calibration_url: str
    mssql_connection_string: str
    oracle_connection_string: str
    temporal_target: str
    smtp_host: str
    smtp_port: int
    oauth_issuer_url: str
    oauth_tls_ca_pem: str = ""
    oauth_tls_ca_file: str = ""
    oauth_required_access_token_scopes: str = ""
    aws_endpoint_url: str = ""
    aws_default_region: str = "us-east-1"
    aws_access_key_id: str = "test"
    aws_secret_access_key: str = "test"
    event_bus_name: str
    event_source: str
    reading_created_detail_type: str = "ReadingCreated"

    @model_validator(mode="after")
    def _oauth_ca_source(self):
        if not self.oauth_tls_ca_pem.strip() and not self.oauth_tls_ca_file.strip():
            raise ValueError("oauth_tls_ca_pem or oauth_tls_ca_file is required")
        return self
