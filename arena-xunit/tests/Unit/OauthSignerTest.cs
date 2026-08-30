using ArenaDotnet.Xunit.Dep;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class OauthSignerTest
{
    [Fact]
    public void Sign_ProviderAndClaimsJson_DelegatesToProvidedFunction()
    {
        var signer = new OauthSigner((provider, claims) => provider.ToJson() + ":" + claims);

        var jwt = signer.Sign(new Provider.Cognito("pool-a"), "{}");

        Assert.Equal("{\"provider\":\"cognito\",\"pool_id\":\"pool-a\"}:{}", jwt);
    }
}
