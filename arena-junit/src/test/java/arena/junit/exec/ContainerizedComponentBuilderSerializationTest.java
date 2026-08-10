package arena.junit.exec;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import com.fasterxml.jackson.databind.node.ObjectNode;

import org.junit.jupiter.api.Test;

final class ContainerizedComponentBuilderSerializationTest {

  @Test
  void build_withBindMount_serializesCorrectJson() {
    ObjectNode config =
        new ContainerizedComponentBuilder("test", "./Dockerfile")
            .withBindMount("/host/data", "/mnt/data", true)
            .build()
            .forFfi();
    assertEquals(1, config.get("bind_mounts").size());
    assertEquals("/host/data", config.get("bind_mounts").get(0).get("host_path").asText());
    assertEquals("/mnt/data", config.get("bind_mounts").get(0).get("container_path").asText());
    assertEquals(true, config.get("bind_mounts").get(0).get("read_only").asBoolean());
  }

  @Test
  void build_withBindMountNoReadOnlyArg_defaultsReadOnlyToFalse() {
    ObjectNode config =
        new ContainerizedComponentBuilder("test", "./Dockerfile")
            .withBindMount("/host/data", "/mnt/data")
            .build()
            .forFfi();
    assertFalse(config.get("bind_mounts").get(0).get("read_only").asBoolean());
  }

  @Test
  void build_withoutBindMount_serializesEmptyList() {
    ObjectNode config = new ContainerizedComponentBuilder("test", "./Dockerfile").build().forFfi();
    assertEquals(0, config.get("bind_mounts").size());
  }
}
