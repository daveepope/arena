using ArenaDotnet.Xunit.Dep;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class OauthSignerTest
{
    [Fact]
    public void Sign_ClaimsJson_DelegatesToProvidedFunction()
    {
        var signer = new OauthSigner(claims => "fake-jwt:" + claims);

        var jwt = signer.Sign("{}");

        Assert.Equal("fake-jwt:{}", jwt);
    }
}
