package dev.arena.junit.dep;
public enum KafkaFlavor {
  APACHE_NATIVE("apache_native"),
  CONFLUENT("confluent");

  private final String value;

  KafkaFlavor(String value) {
    this.value = value;
  }

  public String value() {
    return value;
  }
}
