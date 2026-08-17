using ArenaDotnet.Xunit.Ffi;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class OauthLoopbackTlsTest
{
    [Fact]
    public void OauthLoopbackTlsPemPair_Invoked_ReturnsNonEmptyCertificateAndKey()
    {
        var pair = OauthLoopbackTls.OauthLoopbackTlsPemPair();

        Assert.False(string.IsNullOrEmpty(pair.CertificatePem));
        Assert.False(string.IsNullOrEmpty(pair.PrivateKeyPem));
    }
}
