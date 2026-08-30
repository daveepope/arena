using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.Dep;

public abstract class Provider
{
    public abstract JObject ToJObject();

    public string ToJson() => ToJObject().ToString(Formatting.None);

    public sealed class Cognito : Provider
    {
        public string PoolId { get; }

        public Cognito(string poolId)
        {
            PoolId = poolId;
        }

        public override JObject ToJObject() => new() { ["provider"] = "cognito", ["pool_id"] = PoolId };
    }

    public sealed class Okta : Provider
    {
        public override JObject ToJObject() => new() { ["provider"] = "okta" };
    }

    public sealed class EntraId : Provider
    {
        public string TenantId { get; }

        public EntraId(string tenantId)
        {
            TenantId = tenantId;
        }

        public override JObject ToJObject() => new() { ["provider"] = "entra_id", ["tenant_id"] = TenantId };
    }

    public sealed class Custom : Provider
    {
        public string? IssuerPath { get; }

        public Custom(string? issuerPath = null)
        {
            IssuerPath = issuerPath;
        }

        public override JObject ToJObject()
        {
            var obj = new JObject { ["provider"] = "custom" };
            if (IssuerPath != null)
                obj["issuer_path"] = IssuerPath;
            return obj;
        }
    }
}
