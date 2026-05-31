package arena.examples.readings.component;

import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.playbook.ManagedLocalstackPlaybook;
import arena.junit.playbook.ManagedMssqlPlaybook;
import java.util.List;
import java.util.Map;

final class ReadingsPlaybooks {
  private ReadingsPlaybooks() {}

  static final class CalibrationDefaultPlaybook extends ManagedHttpPlaybook {
    CalibrationDefaultPlaybook(String dependencyIdentifier) {
      super(
          "readings-api-calibration-default",
          dependencyIdentifier,
          List.of(
              mapping(
                  "POST",
                  ReadingsArenaFixture.CALIBRATION_VALIDATE_PATH,
                  200,
                  Map.of("valid", true),
                  Expect.calledAtLeast(1))));
    }
  }

  static final class ValidationDbPlaybook extends ManagedMssqlPlaybook {
    ValidationDbPlaybook(String dependencyIdentifier) {
      super("readings-api-validation-db-scoped", dependencyIdentifier);
    }
  }

  static final class LocalstackSessionPlaybook extends ManagedLocalstackPlaybook {
    LocalstackSessionPlaybook(String dependencyIdentifier) {
      super("readings-api-localstack-session", dependencyIdentifier);
    }
  }
}
