package arena.junit.playbook;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.List;
import org.junit.jupiter.api.Test;

final class ManagedHttpPlaybookUnitTest {

  static final class SequenceBuiltPlaybook extends ManagedHttpPlaybook {
    SequenceBuiltPlaybook() {
      super(
          "pb-seq",
          "dep-http",
          new HttpPlaybookBuilder("dep-http").get("/api/x").willReturn(HttpResponse.ok()));
    }
  }

  @Test
  void expect_called_setsExactlyKindAndCount() {
    ManagedHttpPlaybook.Expect expect = ManagedHttpPlaybook.Expect.called(3);
    assertEquals("exactly", expect.kind());
    assertEquals(3L, expect.count());
  }

  @Test
  void expect_calledAtLeast_setsAtLeastKindAndCount() {
    ManagedHttpPlaybook.Expect expect = ManagedHttpPlaybook.Expect.calledAtLeast(2);
    assertEquals("at_least", expect.kind());
    assertEquals(2L, expect.count());
  }

  @Test
  void expect_neverCalled_setsNeverKindAndNullCount() {
    ManagedHttpPlaybook.Expect expect = ManagedHttpPlaybook.Expect.neverCalled();
    assertEquals("never", expect.kind());
    assertNull(expect.count());
  }

  @Test
  void mapping_nullMethod_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedHttpPlaybook.Mapping(null, "/x", 200));
  }

  @Test
  void mapping_emptyUrlPath_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedHttpPlaybook.Mapping("GET", "", 200));
  }

  @Test
  void mapping_statusOnlyOverload_defaultsJsonBodyAndExpectToNull() {
    ManagedHttpPlaybook.Mapping mapping = ManagedHttpPlaybook.mapping("GET", "/x", 204);
    assertEquals(204, mapping.status());
    assertNull(mapping.jsonBody());
    assertNull(mapping.expect());
  }

  @Test
  void mapping_withJsonBodyOverload_defaultsExpectToNull() {
    ManagedHttpPlaybook.Mapping mapping =
        ManagedHttpPlaybook.mapping("POST", "/x", 201, java.util.Map.of("ok", true));
    assertEquals(java.util.Map.of("ok", true), mapping.jsonBody());
    assertNull(mapping.expect());
  }

  @Test
  void mapping_withJsonBodyAndExpectOverload_carriesBothFields() {
    ManagedHttpPlaybook.Mapping mapping =
        ManagedHttpPlaybook.mapping(
            "POST", "/x", 201, java.util.Map.of("ok", true), ManagedHttpPlaybook.Expect.called(1));
    assertEquals(java.util.Map.of("ok", true), mapping.jsonBody());
    assertEquals("exactly", mapping.expect().kind());
  }

  @Test
  void mapping_withExpectOnlyOverload_defaultsJsonBodyToNull() {
    ManagedHttpPlaybook.Mapping mapping =
        ManagedHttpPlaybook.mapping("GET", "/x", 200, ManagedHttpPlaybook.Expect.neverCalled());
    assertNull(mapping.jsonBody());
    assertEquals("never", mapping.expect().kind());
  }

  @Test
  void constructor_legacyMappingsNull_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class,
        () -> new HttpPlaybookRegistrationLegacyProbe(null));
  }

  @Test
  void constructor_legacyMappingsEmpty_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class,
        () -> new HttpPlaybookRegistrationLegacyProbe(List.of()));
  }

  static final class HttpPlaybookRegistrationLegacyProbe extends ManagedHttpPlaybook {
    HttpPlaybookRegistrationLegacyProbe(List<Mapping> mappings) {
      super("pb-legacy-probe", "dep-http", mappings);
    }
  }

  @Test
  void constructor_sequenceBuilder_forRegisteredFfiReflectsSingleMapping() {
    SequenceBuiltPlaybook playbook = new SequenceBuiltPlaybook();
    assertEquals(1, playbook.forRegisteredFfi().path("mappings").size());
  }

  @Test
  void fromBuilder_functionReturnsUnsupportedType_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class,
        () -> ManagedHttpPlaybook.fromBuilder("pb", "dep-http", builder -> "not-a-builder"));
  }

  @Test
  void fromBuilder_functionReturnsSequenceBuilder_buildsPlaybook() {
    ManagedHttpPlaybook playbook =
        ManagedHttpPlaybook.fromBuilder(
            "pb-from-seq",
            "dep-http",
            b -> b.get("/api/x").willReturn(HttpResponse.ok()));
    assertEquals("pb-from-seq", playbook.identifier());
  }
}
