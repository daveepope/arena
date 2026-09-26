import pytest

from arena_pytest.oauth import Cognito, Custom, EntraId, Okta


@pytest.mark.parametrize(
    "provider,expected",
    [
        (Cognito("us-east-1_abc123"), {"provider": "cognito", "pool_id": "us-east-1_abc123"}),
        (Okta(), {"provider": "okta"}),
        (EntraId("my-tenant"), {"provider": "entra_id", "tenant_id": "my-tenant"}),
        (Custom(), {"provider": "custom"}),
        (Custom("/custom"), {"provider": "custom", "issuer_path": "/custom"}),
    ],
)
def test_to_json_matches_registration_provider_shape(provider, expected):
    assert provider.to_json() == expected


def test_cognito_equal_pool_ids_compare_equal():
    assert Cognito("pool-a") == Cognito("pool-a")


def test_cognito_different_pool_ids_compare_not_equal():
    assert Cognito("pool-a") != Cognito("pool-b")
