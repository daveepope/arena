using System;
using ArenaDotnet.Xunit;

namespace ArenaDotnet.Xunit.Dep;

public sealed class OauthSigner
{
    private readonly Func<string, string> _sign;

    internal OauthSigner(Func<string, string> sign)
    {
        _sign = sign;
    }

    public string Sign(string claimsJson) => _sign(claimsJson);

    public static OauthSigner ForFixture(ArenaCollectionFixture fixture, uint issuerIndex = 0)
    {
        var dependency = fixture.GetDependency<OauthDependency>();
        return new OauthSigner(claims => dependency.SignClaims(fixture.Arena, issuerIndex, claims));
    }
}
