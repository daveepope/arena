package arena.junit;

import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.exec.ExecutableComponent;
import arena.junit.exec.ExecutableComponentBuilder;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.concurrent.TimeUnit;

import org.junit.jupiter.api.Test;

final class ChildrenLifecycleComponentTest {

  private static final String MATCH_NAME = "children-lifecycle-probe";

  @Test
  void open_withChildComponent_startsBothParentAndChild() throws Exception {
    Path markerFile = Files.createTempFile("arena-junit-children", ".txt");
    try {
      ExecutableComponent child =
          new ExecutableComponentBuilder("child")
              .withExecutablePath("/bin/sh")
              .withRuntimeArg("flag", "-c")
              .withRuntimeArg("script", "echo child >> " + markerFile)
              .build();

      ExecutableComponent parent =
          new ExecutableComponentBuilder("parent")
              .withExecutablePath("/bin/sh")
              .withRuntimeArg("flag", "-c")
              .withRuntimeArg("script", "echo parent >> " + markerFile)
              .addChildComponent(child)
              .build();

      Match match = new MatchBuilder(MATCH_NAME).addComponent(parent).build();
      ClosedArena closedArena = new ClosedArena(MATCH_NAME, List.of(match));
      OpenArena arena = closedArena.open();
      try {
        assertNotNull(arena);
        List<String> lines = waitForMarkerLines(markerFile, 5000);
        assertTrue(lines.contains("child"), lines::toString);
        assertTrue(lines.contains("parent"), lines::toString);
      } finally {
        arena.close();
      }
    } finally {
      Files.deleteIfExists(markerFile);
    }
  }

  private static List<String> waitForMarkerLines(Path markerFile, long timeoutMs)
      throws IOException, InterruptedException {
    long deadline = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(timeoutMs);
    while (true) {
      List<String> lines = Files.readAllLines(markerFile);
      if (lines.contains("child") && lines.contains("parent")) {
        return lines;
      }
      if (System.nanoTime() >= deadline) {
        return lines;
      }
      TimeUnit.MILLISECONDS.sleep(20);
    }
  }
}
