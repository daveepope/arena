package arena.junit.support;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

final class ArenaIdentifiersUnitTest {

  @Test
  void build_plainName_prefixesModuleAndSuffixesRandomToken() {
    String id = ArenaIdentifiers.build("arena-postgres", "orders");
    assertTrue(id.matches("arena-postgres-orders-[0-9a-z]{6}"));
  }

  @Test
  void build_emptyName_omitsSlugSegment() {
    String id = ArenaIdentifiers.build("arena-postgres", "");
    assertTrue(id.matches("arena-postgres-[0-9a-z]{6}"));
  }

  @Test
  void build_nameWithSymbolsAndSpaces_slugifiesToDashSeparatedToken() {
    String id = ArenaIdentifiers.build("arena-postgres", "Orders DB!! v2");
    assertTrue(id.matches("arena-postgres-orders-db-v2-[0-9a-z]{6}"));
  }

  @Test
  void build_nameAllSymbols_slugifiesToEmptySegment() {
    String id = ArenaIdentifiers.build("arena-postgres", "!!!");
    assertTrue(id.matches("arena-postgres-[0-9a-z]{6}"));
  }

  @Test
  void build_nameAlreadyHasValidSuffix_returnsNameUnchanged() {
    String alreadySuffixed = "custom-name-abc123";
    assertEquals(alreadySuffixed, ArenaIdentifiers.build("arena-postgres", alreadySuffixed));
  }

  @Test
  void build_calledTwiceWithSameName_producesDifferentSuffixes() {
    String first = ArenaIdentifiers.build("arena-postgres", "orders");
    String second = ArenaIdentifiers.build("arena-postgres", "orders");
    assertTrue(!first.equals(second));
  }
}
