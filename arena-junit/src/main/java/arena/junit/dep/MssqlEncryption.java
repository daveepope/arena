package arena.junit.dep;
public enum MssqlEncryption {
  OFF("off"),
  ON("on");

  private final String value;

  MssqlEncryption(String value) {
    this.value = value;
  }

  public String value() {
    return value;
  }
}
