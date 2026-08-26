using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.Ffi;

public sealed class OauthLoopbackTlsPemPair
{
    public string CertificatePem { get; }
    public string PrivateKeyPem { get; }

    internal OauthLoopbackTlsPemPair(string certificatePem, string privateKeyPem)
    {
        CertificatePem = certificatePem;
        PrivateKeyPem = privateKeyPem;
    }
}

public static class OauthLoopbackTls
{
    public static OauthLoopbackTlsPemPair OauthLoopbackTlsPemPair()
    {
        var raw = ArenaBindings.OauthLoopbackTlsPemJson();
        var obj = JObject.Parse(raw);
        var cert = obj["certificate_pem"]?.Value<string>();
        var key = obj["private_key_pem"]?.Value<string>();
        if (cert is null || cert.Length == 0 || key is null || key.Length == 0)
        {
            throw new ArenaBindingError(
                "arena_oauth_loopback_tls_pem_json: missing certificate_pem or private_key_pem");
        }
        return new OauthLoopbackTlsPemPair(cert, key);
    }
}
