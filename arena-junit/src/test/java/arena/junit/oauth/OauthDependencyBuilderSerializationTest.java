package arena.junit.oauth;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.match.ArenaRunnableDependency;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.stream.Stream;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

final class OauthDependencyBuilderSerializationTest {

  static final class StubDependency implements ArenaRunnableDependency {
    @Override
    public ObjectNode forFfi() {
      return ArenaJson.object().put("identifier", "child");
    }
  }

  @Test
  void build_minimalName_serializesTypeAndDefaultPort() {
    ObjectNode config = new OauthDependencyBuilder("oauth").build().forFfi();
    assertEquals("oauth", config.path("type").asText());
    assertTrue(config.path("identifier").asText().startsWith("arena-oauth-oauth-"));
    assertEquals(OauthDependencyBuilder.DEFAULT_OAUTH_PORT, config.path("port").asInt());
    assertFalse(config.has("children"));
  }

  @Test
  void build_noMetadataBaseUrl_defaultsToOauthIssuer() {
    ObjectNode config = new OauthDependencyBuilder("oauth").build().forFfi();
    assertEquals(OauthDependencyBuilder.OAUTH_ISSUER, config.path("metadata_base_url").asText());
  }

  @Test
  void withMetadataBaseUrl_trailingSlashes_stripped() {
    ObjectNode config =
        new OauthDependencyBuilder("oauth")
            .withMetadataBaseUrl("https://issuer.example.com///")
            .build()
            .forFfi();
    assertEquals("https://issuer.example.com", config.path("metadata_base_url").asText());
  }

  @Test
  void withPortAndListenIp_setsScalarFields() {
    ObjectNode config =
        new OauthDependencyBuilder("oauth").withPort(9555).withListenIp("0.0.0.0").build().forFfi();
    assertEquals(9555, config.path("port").asInt());
    assertEquals("0.0.0.0", config.path("listen_ip").asText());
  }

  @Test
  void withServerTlsPem_setsCertKeyAndTlsTransport() {
    ObjectNode config =
        new OauthDependencyBuilder("oauth")
            .withServerTlsPem("cert-pem", "key-pem")
            .build()
            .forFfi();
    assertEquals("cert-pem", config.path("server_tls_certificate_pem").asText());
    assertEquals("key-pem", config.path("server_tls_private_key_pem").asText());
    assertEquals("tls", config.path("transport").asText());
  }

  @Test
  void withHttp_setsHttpTransport() {
    ObjectNode config = new OauthDependencyBuilder("oauth").withHttp().build().forFfi();
    assertEquals("http", config.path("transport").asText());
  }

  @Test
  void addChildDependency_nonEmptyChildren_serializesChildrenArray() {
    ObjectNode config =
        new OauthDependencyBuilder("oauth").addChildDependency(new StubDependency()).build().forFfi();
    assertEquals(1, ((ArrayNode) config.path("children")).size());
  }

  @Test
  void identifier_returnsConfiguredIdentifier() {
    OauthDependency dep = new OauthDependencyBuilder("oauth").build();
    assertTrue(dep.identifier().startsWith("arena-oauth-oauth-"));
  }

  @Test
  void withIssuerCognito_poolId_appendsCognitoProviderEntry() {
    ObjectNode config =
        new OauthDependencyBuilder("oauth").withIssuerCognito("us-east-1_abc123").build().forFfi();
    ArrayNode issuers = (ArrayNode) config.path("issuers");
    assertEquals(1, issuers.size());
    assertEquals("cognito", issuers.get(0).path("provider").asText());
    assertEquals("us-east-1_abc123", issuers.get(0).path("pool_id").asText());
  }

  @Test
  void withIssuerOkta_appendsOktaProviderEntry() {
    ObjectNode config = new OauthDependencyBuilder("oauth").withIssuerOkta().build().forFfi();
    ArrayNode issuers = (ArrayNode) config.path("issuers");
    assertEquals(1, issuers.size());
    assertEquals("okta", issuers.get(0).path("provider").asText());
  }

  @Test
  void withIssuerEntraId_tenantId_appendsEntraIdProviderEntry() {
    ObjectNode config =
        new OauthDependencyBuilder("oauth").withIssuerEntraId("my-tenant").build().forFfi();
    ArrayNode issuers = (ArrayNode) config.path("issuers");
    assertEquals(1, issuers.size());
    assertEquals("entra_id", issuers.get(0).path("provider").asText());
    assertEquals("my-tenant", issuers.get(0).path("tenant_id").asText());
  }

  static Stream<Arguments> customIssuerFields() {
    return Stream.of(
        Arguments.of("/custom", "/custom/keys", null),
        Arguments.of(null, "/v1/keys", null),
        Arguments.of(null, null, "pkcs8-pem-placeholder"));
  }

  @ParameterizedTest
  @MethodSource("customIssuerFields")
  void withIssuer_customFields_serializesOnlySuppliedFields(
      String issuerPath, String jwksPath, String rsaPkcs8Pem) {
    ObjectNode config =
        new OauthDependencyBuilder("oauth")
            .withIssuer(issuerPath, jwksPath, rsaPkcs8Pem)
            .build()
            .forFfi();
    ObjectNode entry = (ObjectNode) ((ArrayNode) config.path("issuers")).get(0);
    assertEquals("custom", entry.path("provider").asText());
    assertEquals(issuerPath != null, entry.has("issuer_path"));
    assertEquals(jwksPath != null, entry.has("jwks_path"));
    assertEquals(rsaPkcs8Pem != null, entry.has("rsa_pkcs8_pem"));
  }

  @Test
  void withIssuerCalls_multipleProviders_accumulateInOrder() {
    ObjectNode config =
        new OauthDependencyBuilder("oauth")
            .withIssuerCognito("pool-a")
            .withIssuerOkta()
            .build()
            .forFfi();
    ArrayNode issuers = (ArrayNode) config.path("issuers");
    assertEquals(2, issuers.size());
    assertEquals("cognito", issuers.get(0).path("provider").asText());
    assertEquals("okta", issuers.get(1).path("provider").asText());
  }
}
