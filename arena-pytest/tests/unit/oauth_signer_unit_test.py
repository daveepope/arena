import asyncio

from arena_pytest.oauth import Cognito, OauthSigner, _build_oauth_signer


class FakeOauthDependency:
    def __init__(self):
        self.calls = []

    async def sign_claims(self, arena, provider, claims_json):
        self.calls.append((arena, provider, claims_json))
        return "fake-jwt"


def test_sign_delegates_to_oauth_dependency_sign_claims_with_bound_arena_and_provider():
    fake_dependency = FakeOauthDependency()
    fake_arena = object()
    signer = OauthSigner(fake_dependency, fake_arena)
    provider = Cognito("pool-a")

    jwt = asyncio.run(signer.sign(provider, '{"sub":"test-user"}'))

    assert jwt == "fake-jwt"
    assert fake_dependency.calls == [(fake_arena, provider, '{"sub":"test-user"}')]


def test_build_oauth_signer_wraps_lazily_resolved_dependency_and_arena():
    fake_dependency = FakeOauthDependency()
    fake_arena = object()
    provider = Cognito("pool-b")

    signer = _build_oauth_signer(lambda: fake_dependency, fake_arena)
    asyncio.run(signer.sign(provider, "{}"))

    assert fake_dependency.calls == [(fake_arena, provider, "{}")]


def test_build_oauth_signer_calls_getter_each_time():
    calls = []

    def getter():
        calls.append(1)
        return FakeOauthDependency()

    _build_oauth_signer(getter, object())

    assert calls == [1]
