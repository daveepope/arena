package arena.junit.ffi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

final class ArenaStatusUnitTest {

  static Stream<Arguments> fromInt_knownCode_returnsMatchingStatusSource() {
    return Stream.of(
        Arguments.of(0, ArenaStatus.OK),
        Arguments.of(1, ArenaStatus.INVALID_ARGUMENT),
        Arguments.of(2, ArenaStatus.FAILED),
        Arguments.of(3, ArenaStatus.PANIC),
        Arguments.of(4, ArenaStatus.NOT_FOUND));
  }

  @ParameterizedTest
  @MethodSource("fromInt_knownCode_returnsMatchingStatusSource")
  void fromInt_knownCode_returnsMatchingStatus(int code, ArenaStatus expected) {
    assertEquals(expected, ArenaStatus.fromInt(code));
    assertEquals(code, expected.code());
  }

  @Test
  void fromInt_unknownCode_throwsIllegalArgumentException() {
    assertThrows(IllegalArgumentException.class, () -> ArenaStatus.fromInt(99));
  }
}
