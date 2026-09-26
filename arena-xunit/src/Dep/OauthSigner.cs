using System;
using ArenaDotnet.Xunit;

namespace ArenaDotnet.Xunit.Dep;

public sealed class OauthSigner
{
    private readonly Func<Provider, string, string> _sign;

    internal OauthSigner(Func<Provider, string, string> sign)
    {
        _sign = sign;
    }

    public string Sign(Provider provider, string claimsJson) => _sign(provider, claimsJson);

    public static OauthSigner ForFixture(ArenaCollectionFixture fixture)
    {
        var dependency = fixture.GetDependency<OauthDependency>();
        return new OauthSigner((provider, claims) => dependency.SignClaims(fixture.Arena, provider, claims));
    }
}
