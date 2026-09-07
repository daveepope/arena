package arena.junit.lifecycle;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class ArenaStateUnitTest {

  static final String FIXTURE_STATE_JSON =
      "{"
          + "\"id\":\"orders\",\"state\":\"arena_faulted\",\"at\":\"2026-09-06T11:02:44.812Z\","
          + "\"dependencies\":[{\"id\":\"orders-postgres\",\"state\":\"faulted\","
          + "\"faults\":[{\"id\":\"orders-postgres\",\"subject\":\"dependency\","
          + "\"message\":\"failed to start\",\"at\":\"2026-09-06T11:02:44.801Z\","
          + "\"faults\":[{\"id\":\"orders-postgres\",\"subject\":\"dependency\","
          + "\"message\":\"connection refused on 127.0.0.1:5432\","
          + "\"at\":\"2026-09-06T11:02:44.799Z\",\"faults\":[]}]}],"
          + "\"children\":[{\"id\":\"orders-postgres-seed\",\"state\":\"stopped\","
          + "\"faults\":[],\"children\":[]}]}],"
          + "\"components\":[{\"id\":\"orders-api\",\"state\":\"not_started\","
          + "\"faults\":[],\"children\":[]}],"
          + "\"faults\":[{\"id\":\"orders-postgres\",\"subject\":\"dependency\","
          + "\"message\":\"failed to start\",\"at\":\"2026-09-06T11:02:44.801Z\",\"faults\":[]}]"
          + "}";

  @Test
  void parseFixtureDocumentRoundTripsEveryField() {
    ArenaState state = ArenaState.parse(FIXTURE_STATE_JSON);

    assertEquals("orders", state.id);
    assertEquals("arena_faulted", state.state);
    assertEquals("2026-09-06T11:02:44.812Z", state.at);
    assertEquals("orders-postgres", state.dependencies.get(0).id);
    assertEquals("faulted", state.dependencies.get(0).state);
    assertEquals("orders-postgres-seed", state.dependencies.get(0).children.get(0).id);
    assertEquals("not_started", state.components.get(0).state);
    assertEquals("dependency", state.faults.get(0).subject);
    Fault cause = state.dependencies.get(0).faults.get(0).faults.get(0);
    assertEquals("connection refused on 127.0.0.1:5432", cause.message);
    assertEquals("2026-09-06T11:02:44.799Z", cause.at);
  }

  @Test
  void parseNonObjectDocumentThrows() {
    assertThrows(IllegalArgumentException.class, () -> ArenaState.parse("[1, 2]"));
    assertThrows(IllegalArgumentException.class, () -> ArenaState.parse("{broken"));
  }

  @Test
  void parseMissingCollectionsDefaultsToEmpty() {
    ArenaState state = ArenaState.parse("{\"id\":\"bare\",\"state\":\"arena_created\",\"at\":\"t\"}");

    assertTrue(state.dependencies.isEmpty());
    assertTrue(state.components.isEmpty());
    assertTrue(state.faults.isEmpty());
  }

  @Test
  void parseUnknownFieldIsTolerated() {
    ArenaState state =
        ArenaState.parse("{\"id\":\"fwd\",\"state\":\"arena_open\",\"at\":\"t\",\"later\":1}");

    assertEquals("fwd", state.id);
  }

  @Test
  void isFaultedFaultedTokenReturnsTrue() {
    assertTrue(ArenaState.parse(FIXTURE_STATE_JSON).isFaulted());
    assertFalse(
        ArenaState.parse("{\"id\":\"x\",\"state\":\"arena_open\",\"at\":\"t\"}").isFaulted());
  }

  @Test
  void dependencyNestedChildIdentifierReturnsThatChild() {
    ArenaState state = ArenaState.parse(FIXTURE_STATE_JSON);

    DependencyState child = state.dependency("orders-postgres-seed");

    assertEquals("stopped", child.state);
    assertNull(state.dependency("nope"));
  }

  @Test
  void componentTopLevelIdentifierReturnsThatComponent() {
    assertEquals("not_started", ArenaState.parse(FIXTURE_STATE_JSON).component("orders-api").state);
  }

  @Test
  void componentNestedChildIdentifierReturnsThatChild() {
    ArenaState state =
        ArenaState.parse(
            "{\"id\":\"nested\",\"state\":\"arena_open\",\"at\":\"t\","
                + "\"components\":[{\"id\":\"api\",\"state\":\"started\",\"faults\":[],"
                + "\"children\":[{\"id\":\"worker\",\"state\":\"started\","
                + "\"faults\":[],\"children\":[]}]}]}");

    assertEquals("started", state.component("worker").state);
    assertNull(state.component("nope"));
  }
}
