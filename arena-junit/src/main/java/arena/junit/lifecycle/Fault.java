package arena.junit.lifecycle;

import java.util.List;

public final class Fault {
  public String id = "";
  public String subject = "";
  public String message = "";
  public String at = "";
  public List<Fault> faults = List.of();
}
