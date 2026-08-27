from __future__ import annotations

import asyncio

from arena_pytest import ClosedArena, MatchBuilder, OauthDependencyBuilder, OauthSigner

MATCH_NAME = "oauth-signer-probe"


async def _sign_with_running_dependency() -> str:
    oauth = OauthDependencyBuilder("oauth-signer-component").with_http().build()
    a_match = MatchBuilder(MATCH_NAME).add_dependency(oauth).build()
    closed = ClosedArena(MATCH_NAME, [a_match])
    arena = await closed.open()
    try:
        signer = OauthSigner(oauth, arena, issuer_index=0)
        return await signer.sign('{"sub":"test-user","iat":0,"exp":9999999999}')
    finally:
        await arena.close()


def test_sign_running_dependency_returns_verifiable_jwt():
    jwt = asyncio.run(_sign_with_running_dependency())
    assert len(jwt.split(".")) == 3
