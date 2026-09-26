import pytest

from arena_pytest.oauth import OauthDependencyBuilder


def test_with_issuer_cognito_appends_cognito_provider_entry():
    config = OauthDependencyBuilder("oauth").with_issuer_cognito("us-east-1_abc123").build()._for_ffi()
    assert config["issuers"] == [{"provider": "cognito", "pool_id": "us-east-1_abc123"}]


def test_with_issuer_okta_appends_okta_provider_entry():
    config = OauthDependencyBuilder("oauth").with_issuer_okta().build()._for_ffi()
    assert config["issuers"] == [{"provider": "okta"}]


def test_with_issuer_entra_id_appends_entra_id_provider_entry():
    config = OauthDependencyBuilder("oauth").with_issuer_entra_id("my-tenant").build()._for_ffi()
    assert config["issuers"] == [{"provider": "entra_id", "tenant_id": "my-tenant"}]


@pytest.mark.parametrize(
    "kwargs,expected",
    [
        (
            {"issuer_path": "/custom", "jwks_path": "/custom/keys"},
            {"provider": "custom", "issuer_path": "/custom", "jwks_path": "/custom/keys"},
        ),
        (
            {"jwks_path": "/v1/keys"},
            {"provider": "custom", "jwks_path": "/v1/keys"},
        ),
        (
            {"rsa_pkcs8_pem": "pkcs8-pem-placeholder"},
            {"provider": "custom", "rsa_pkcs8_pem": "pkcs8-pem-placeholder"},
        ),
    ],
)
def test_with_issuer_custom_appends_only_supplied_fields(kwargs, expected):
    config = OauthDependencyBuilder("oauth").with_issuer(**kwargs).build()._for_ffi()
    assert config["issuers"] == [expected]


def test_with_issuer_calls_accumulate_in_order():
    config = (
        OauthDependencyBuilder("oauth")
        .with_issuer_cognito("pool-a")
        .with_issuer_okta()
        .build()
        ._for_ffi()
    )
    assert config["issuers"] == [
        {"provider": "cognito", "pool_id": "pool-a"},
        {"provider": "okta"},
    ]
