package dev.arena.examples.readings.springboot;

import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.KeyStore;
import java.security.cert.Certificate;
import java.security.cert.CertificateFactory;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import javax.net.ssl.SSLContext;
import javax.net.ssl.TrustManagerFactory;

public final class PemTrust {

  private PemTrust() {}

  public static SSLContext sslContextFromCaPem(String pem) throws Exception {
    return sslContextFromPemBytes(pem.getBytes(StandardCharsets.UTF_8));
  }

  public static SSLContext sslContextFromCaFile(Path path) throws Exception {
    return sslContextFromPemBytes(Files.readAllBytes(path));
  }

  private static SSLContext sslContextFromPemBytes(byte[] pemBytes) throws Exception {
    String pem = new String(pemBytes, StandardCharsets.UTF_8);
    List<byte[]> derBlocks = new ArrayList<>();
    int i = 0;
    while (true) {
      int b = pem.indexOf("-----BEGIN CERTIFICATE-----", i);
      if (b < 0) {
        break;
      }
      int e = pem.indexOf("-----END CERTIFICATE-----", b);
      if (e < 0) {
        throw new IllegalArgumentException("malformed PEM");
      }
      String block =
          pem.substring(b + "-----BEGIN CERTIFICATE-----".length(), e).replaceAll("\\s", "");
      derBlocks.add(Base64.getDecoder().decode(block));
      i = e + "-----END CERTIFICATE-----".length();
    }
    if (derBlocks.isEmpty()) {
      throw new IllegalArgumentException("no certificates in PEM");
    }
    CertificateFactory cf = CertificateFactory.getInstance("X.509");
    KeyStore ks = KeyStore.getInstance(KeyStore.getDefaultType());
    ks.load(null);
    int n = 0;
    for (byte[] der : derBlocks) {
      Certificate cert = cf.generateCertificate(new ByteArrayInputStream(der));
      ks.setCertificateEntry("c" + n++, cert);
    }
    TrustManagerFactory tmf =
        TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm());
    tmf.init(ks);
    SSLContext ctx = SSLContext.getInstance("TLS");
    ctx.init(null, tmf.getTrustManagers(), null);
    return ctx;
  }
}
