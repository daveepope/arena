package arena.junit.exec;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

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
    assertEquals(1, config.get("mounts").size());
    assertEquals("bind", config.get("mounts").get(0).get("type").asText());
    assertEquals("/host/data", config.get("mounts").get(0).get("source").asText());
    assertEquals("/mnt/data", config.get("mounts").get(0).get("container_path").asText());
    assertEquals(true, config.get("mounts").get(0).get("read_only").asBoolean());
  }

  @Test
  void build_withBindMountNoReadOnlyArg_defaultsReadOnlyToFalse() {
    ObjectNode config =
        new ContainerizedComponentBuilder("test", "./Dockerfile")
            .withBindMount("/host/data", "/mnt/data")
            .build()
            .forFfi();
    assertFalse(config.get("mounts").get(0).get("read_only").asBoolean());
  }

  @Test
  void build_withVolumeMount_serializesCorrectJson() {
    ObjectNode config =
        new ContainerizedComponentBuilder("test", "./Dockerfile")
            .withVolumeMount("my-volume", "/mnt/data", true)
            .build()
            .forFfi();
    assertEquals(1, config.get("mounts").size());
    assertEquals("volume", config.get("mounts").get(0).get("type").asText());
    assertEquals("my-volume", config.get("mounts").get(0).get("source").asText());
    assertEquals("/mnt/data", config.get("mounts").get(0).get("container_path").asText());
    assertEquals(true, config.get("mounts").get(0).get("read_only").asBoolean());
  }

  @Test
  void build_withTmpfsMount_serializesCorrectJson() {
    ObjectNode config =
        new ContainerizedComponentBuilder("test", "./Dockerfile")
            .withTmpfsMount("/mnt/data", 1024L)
            .build()
            .forFfi();
    assertEquals(1, config.get("mounts").size());
    assertEquals("tmpfs", config.get("mounts").get(0).get("type").asText());
    assertEquals("/mnt/data", config.get("mounts").get(0).get("container_path").asText());
    assertEquals(1024L, config.get("mounts").get(0).get("size_bytes").asLong());
  }

  @Test
  void build_withTmpfsMountNoSizeBytesArg_omitsSizeBytes() {
    ObjectNode config =
        new ContainerizedComponentBuilder("test", "./Dockerfile")
            .withTmpfsMount("/mnt/data")
            .build()
            .forFfi();
    assertTrue(config.get("mounts").get(0).get("size_bytes") == null);
  }

  @Test
  void build_withoutMounts_serializesEmptyList() {
    ObjectNode config = new ContainerizedComponentBuilder("test", "./Dockerfile").build().forFfi();
    assertEquals(0, config.get("mounts").size());
  }
}
