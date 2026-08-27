package arena.junit.oauth;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;

public sealed interface Provider {
  String toJson();

  record Cognito(String poolId) implements Provider {
    @Override
    public String toJson() {
      return ArenaJson.object().put("provider", "cognito").put("pool_id", poolId).toString();
    }
  }

  record Okta() implements Provider {
    @Override
    public String toJson() {
      return ArenaJson.object().put("provider", "okta").toString();
    }
  }

  record EntraId(String tenantId) implements Provider {
    @Override
    public String toJson() {
      return ArenaJson.object().put("provider", "entra_id").put("tenant_id", tenantId).toString();
    }
  }

  record Custom(String issuerPath) implements Provider {
    @Override
    public String toJson() {
      ObjectNode node = ArenaJson.object().put("provider", "custom");
      if (issuerPath != null) {
        node.put("issuer_path", issuerPath);
      }
      return node.toString();
    }
  }
}
