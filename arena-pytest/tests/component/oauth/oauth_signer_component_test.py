from __future__ import annotations

import asyncio

from arena_pytest import (
    ClosedArena,
    Cognito,
    Custom,
    MatchBuilder,
    Okta,
    OauthDependencyBuilder,
    OauthSigner,
)

MATCH_NAME = "oauth-signer-probe"
MIXED_MATCH_NAME = "oauth-signer-mixed-probe"


async def _sign_with_running_dependency() -> str:
    oauth = OauthDependencyBuilder("oauth-signer-component").with_http().build()
    a_match = MatchBuilder(MATCH_NAME).add_dependency(oauth).build()
    closed = ClosedArena(MATCH_NAME, [a_match])
    arena = await closed.open()
    try:
        signer = OauthSigner(oauth, arena)
        return await signer.sign(Custom(), '{"sub":"test-user","iat":0,"exp":9999999999}')
    finally:
        await arena.close()


def test_sign_running_dependency_returns_verifiable_jwt():
    jwt = asyncio.run(_sign_with_running_dependency())
    assert len(jwt.split(".")) == 3


async def _sign_cognito_and_okta_with_mixed_dependency() -> tuple[str, str]:
    oauth = (
        OauthDependencyBuilder("oauth-signer-mixed")
        .with_http()
        .with_issuer_cognito("us-east-1_abc123")
        .with_issuer_okta()
        .build()
    )
    a_match = MatchBuilder(MIXED_MATCH_NAME).add_dependency(oauth).build()
    closed = ClosedArena(MIXED_MATCH_NAME, [a_match])
    arena = await closed.open()
    try:
        signer = OauthSigner(oauth, arena)
        cognito_jwt = await signer.sign(
            Cognito("us-east-1_abc123"), '{"sub":"test-user","iat":0,"exp":9999999999}'
        )
        okta_jwt = await signer.sign(Okta(), '{"sub":"test-user","iat":0,"exp":9999999999}')
        return cognito_jwt, okta_jwt
    finally:
        await arena.close()


def test_sign_with_explicit_provider_selects_matching_issuer_on_mixed_dependency():
    cognito_jwt, okta_jwt = asyncio.run(_sign_cognito_and_okta_with_mixed_dependency())
    assert len(cognito_jwt.split(".")) == 3
    assert len(okta_jwt.split(".")) == 3
    assert cognito_jwt != okta_jwt
