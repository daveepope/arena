package dev.arena.examples.readings.springboot;

import java.nio.file.Path;
import java.util.Arrays;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import javax.net.ssl.SSLContext;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.security.config.annotation.web.builders.HttpSecurity;
import org.springframework.security.config.annotation.web.configuration.EnableWebSecurity;
import org.springframework.security.config.annotation.web.configurers.AbstractHttpConfigurer;
import org.springframework.security.core.authority.SimpleGrantedAuthority;
import org.springframework.security.oauth2.core.OAuth2AuthenticationException;
import org.springframework.security.oauth2.core.OAuth2Error;
import org.springframework.security.oauth2.jwt.Jwt;
import org.springframework.security.oauth2.jwt.JwtDecoder;
import org.springframework.security.oauth2.jwt.JwtDecoders;
import org.springframework.security.oauth2.server.resource.authentication.JwtAuthenticationConverter;
import org.springframework.security.web.SecurityFilterChain;

@Configuration
@EnableWebSecurity
public class JwtResourceServerConfiguration {

  @Bean
  JwtDecoder arenaJwtDecoder(
      @Value("${OAUTH_ISSUER_URL}") String issuerRaw,
      @Value("${OAUTH_TLS_CA_FILE:}") String caFile,
      @Value("${OAUTH_TLS_CA_PEM:}") String caPem)
      throws Exception {
    SSLContext ssl;
    if (caFile != null && !caFile.isBlank()) {
      ssl = PemTrust.sslContextFromCaFile(Path.of(caFile.trim()));
    } else if (caPem != null && !caPem.isBlank()) {
      ssl = PemTrust.sslContextFromCaPem(caPem);
    } else {
      throw new IllegalStateException("OAUTH_TLS_CA_FILE or OAUTH_TLS_CA_PEM required");
    }
    SSLContext prior = SSLContext.getDefault();
    SSLContext.setDefault(ssl);
    try {
      return JwtDecoders.fromIssuerLocation(issuerRaw);
    } finally {
      SSLContext.setDefault(prior);
    }
  }

  @Bean
  JwtAuthenticationConverter arenaJwtAuthenticationConverter(
      @Value("${OAUTH_REQUIRED_ACCESS_TOKEN_SCOPES:}") String scopes) {
    List<String> required =
        Arrays.stream(scopes.split("\\s+")).map(String::trim).filter(s -> !s.isEmpty()).toList();
    JwtAuthenticationConverter c = new JwtAuthenticationConverter();
    c.setJwtGrantedAuthoritiesConverter(
        (Jwt jwt) -> {
          if (required.isEmpty()) {
            return List.of();
          }
          String sc = jwt.getClaimAsString("scope");
          if (sc == null || sc.isBlank()) {
            throw new OAuth2AuthenticationException(new OAuth2Error("invalid_token"));
          }
          Set<String> granted = new HashSet<>(Arrays.asList(sc.split("\\s+")));
          if (!granted.containsAll(required)) {
            throw new OAuth2AuthenticationException(new OAuth2Error("insufficient_scope"));
          }
          return List.of(new SimpleGrantedAuthority("SCOPE_readings"));
        });
    return c;
  }

  @Bean
  SecurityFilterChain arenaSecurityFilterChain(
      HttpSecurity http, JwtDecoder arenaJwtDecoder, JwtAuthenticationConverter arenaJwtAuthenticationConverter)
      throws Exception {
    http.csrf(AbstractHttpConfigurer::disable);
    http.authorizeHttpRequests(
        a -> a.requestMatchers("/health").permitAll().anyRequest().authenticated());
    http.oauth2ResourceServer(
        o ->
            o.jwt(
                j ->
                    j.decoder(arenaJwtDecoder)
                        .jwtAuthenticationConverter(arenaJwtAuthenticationConverter)));
    return http.build();
  }
}
