using ArenaDotnet.Xunit.Dep;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class OauthProviderTest
{
    [Fact]
    public void ToJson_Cognito_ReturnsProviderAndPoolId()
    {
        Provider provider = new Provider.Cognito("us-east-1_abc123");

        Assert.Equal("{\"provider\":\"cognito\",\"pool_id\":\"us-east-1_abc123\"}", provider.ToJson());
    }

    [Fact]
    public void ToJson_Okta_ReturnsProviderOnly()
    {
        Provider provider = new Provider.Okta();

        Assert.Equal("{\"provider\":\"okta\"}", provider.ToJson());
    }

    [Fact]
    public void ToJson_EntraId_ReturnsProviderAndTenantId()
    {
        Provider provider = new Provider.EntraId("my-tenant");

        Assert.Equal("{\"provider\":\"entra_id\",\"tenant_id\":\"my-tenant\"}", provider.ToJson());
    }

    [Fact]
    public void ToJson_CustomWithoutIssuerPath_ReturnsProviderOnly()
    {
        Provider provider = new Provider.Custom();

        Assert.Equal("{\"provider\":\"custom\"}", provider.ToJson());
    }

    [Fact]
    public void ToJson_CustomWithIssuerPath_ReturnsProviderAndIssuerPath()
    {
        Provider provider = new Provider.Custom("/custom");

        Assert.Equal("{\"provider\":\"custom\",\"issuer_path\":\"/custom\"}", provider.ToJson());
    }
}
