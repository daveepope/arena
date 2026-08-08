package arena.junit.playbook;

public interface ManagedPlaybook extends Playbook {
  default boolean activatesBeforeTest() {
    return false;
  }
}
