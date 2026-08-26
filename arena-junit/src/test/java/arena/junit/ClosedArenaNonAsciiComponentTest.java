package arena.junit;

import static org.junit.jupiter.api.Assertions.assertNotNull;

import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import java.util.List;
import org.junit.jupiter.api.Test;

final class ClosedArenaNonAsciiComponentTest {

  @Test
  void open_matchNameWithNonAsciiCharacters_roundTripsThroughFfi() throws Exception {
    String name = "arena-café-☕-日本語";
    Match match = new MatchBuilder(name).build();
    ClosedArena closedArena = new ClosedArena(name, List.of(match));

    OpenArena openArena = closedArena.open();
    try {
      assertNotNull(openArena);
    } finally {
      openArena.close();
    }
  }
}
