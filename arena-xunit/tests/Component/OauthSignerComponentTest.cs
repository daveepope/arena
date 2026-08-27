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
        var jwt = _fixture.Signer.Sign("{\"sub\":\"test-user\",\"iat\":0,\"exp\":9999999999}");

        Assert.Equal(3, jwt.Split('.').Length);
    }
}
