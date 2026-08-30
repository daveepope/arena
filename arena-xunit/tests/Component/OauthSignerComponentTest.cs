using ArenaDotnet.Xunit;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Xunit;
using Xunit;

namespace ArenaDotnet.Xunit.ComponentTest;

public class OauthSignerComponentTest : IClassFixture<OauthSignerComponentTest.Fixture>
{
    private static readonly int _port = TestRuntime.AllocatePort();

    private readonly Fixture _fixture;

    public OauthSignerComponentTest(Fixture fixture)
    {
        _fixture = fixture;
    }

    public class Fixture : ArenaCollectionFixture
    {
        [ArenaDependency]
        private static readonly OauthDependency Oauth =
            new OauthDependencyBuilder("oauth-signer-component")
                .WithPort(_port)
                .WithHttp()
                .Build();
    }

    [Fact]
    internal void Sign_RunningDependency_ReturnsVerifiableJwt()
    {
        var jwt = _fixture.Signer.Sign(new Provider.Custom(), "{\"sub\":\"test-user\",\"iat\":0,\"exp\":9999999999}");

        Assert.Equal(3, jwt.Split('.').Length);
    }
}

public class OauthSignerMixedProvidersComponentTest : IClassFixture<OauthSignerMixedProvidersComponentTest.Fixture>
{
    private static readonly int _port = TestRuntime.AllocatePort();

    private readonly Fixture _fixture;

    public OauthSignerMixedProvidersComponentTest(Fixture fixture)
    {
        _fixture = fixture;
    }

    public class Fixture : ArenaCollectionFixture
    {
        [ArenaDependency]
        private static readonly OauthDependency Oauth =
            new OauthDependencyBuilder("oauth-signer-mixed-component")
                .WithPort(_port)
                .WithHttp()
                .WithIssuerCognito("us-east-1_abc123")
                .WithIssuerOkta()
                .Build();
    }

    [Fact]
    internal void Sign_WithExplicitProvider_SelectsMatchingIssuerOnMixedDependency()
    {
        var claims = "{\"sub\":\"test-user\",\"iat\":0,\"exp\":9999999999}";
        var cognitoJwt = _fixture.Signer.Sign(new Provider.Cognito("us-east-1_abc123"), claims);
        var oktaJwt = _fixture.Signer.Sign(new Provider.Okta(), claims);

        Assert.Equal(3, cognitoJwt.Split('.').Length);
        Assert.Equal(3, oktaJwt.Split('.').Length);
        Assert.NotEqual(cognitoJwt, oktaJwt);
    }
}
