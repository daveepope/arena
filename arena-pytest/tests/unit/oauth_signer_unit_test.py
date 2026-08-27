import asyncio

from arena_pytest.oauth import OauthSigner, _build_oauth_signer


class FakeOauthDependency:
    def __init__(self):
        self.calls = []

    async def sign_claims(self, arena, issuer_index, claims_json):
        self.calls.append((arena, issuer_index, claims_json))
        return "fake-jwt"


def test_sign_delegates_to_oauth_dependency_sign_claims_with_bound_arena_and_index():
    fake_dependency = FakeOauthDependency()
    fake_arena = object()
    signer = OauthSigner(fake_dependency, fake_arena, issuer_index=1)

    jwt = asyncio.run(signer.sign('{"sub":"test-user"}'))

    assert jwt == "fake-jwt"
    assert fake_dependency.calls == [(fake_arena, 1, '{"sub":"test-user"}')]


def test_sign_no_issuer_index_defaults_to_zero():
    fake_dependency = FakeOauthDependency()
    fake_arena = object()
    signer = OauthSigner(fake_dependency, fake_arena)

    asyncio.run(signer.sign("{}"))

    assert fake_dependency.calls[0][1] == 0


def test_build_oauth_signer_wraps_lazily_resolved_dependency_and_arena():
    fake_dependency = FakeOauthDependency()
    fake_arena = object()

    signer = _build_oauth_signer(lambda: fake_dependency, 2, fake_arena)
    asyncio.run(signer.sign("{}"))

    assert fake_dependency.calls == [(fake_arena, 2, "{}")]


def test_build_oauth_signer_calls_getter_each_time():
    calls = []

    def getter():
        calls.append(1)
        return FakeOauthDependency()

    _build_oauth_signer(getter, 0, object())

    assert calls == [1]
