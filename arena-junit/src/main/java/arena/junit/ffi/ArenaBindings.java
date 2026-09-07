package arena.junit.ffi;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.IntByReference;
import com.sun.jna.ptr.PointerByReference;
import java.nio.charset.StandardCharsets;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import org.slf4j.ILoggerFactory;
import org.slf4j.Logger;

public final class ArenaBindings {
  private static final Set<Long> DEFAULT_DISPATCHER_LOGGING_TARGET_TOKENS =
      ConcurrentHashMap.newKeySet();

  private static final Map<Long, ArenaSlf4jDispatcherStderrPublication>
      SLF4J_DISPATCHER_STDERR_PUBLICATIONS = new ConcurrentHashMap<>();

  private ArenaBindings() {}

  public static ArenaNativeLib lib() {
    ArenaNativeLib lib = ArenaNativeHolder.LIB;
    if (lib == null) {
      throw new ArenaBindingError(
          "arena shared library not found (set ARENA_FFI_LIB, use Bazel runfiles, or add the"
              + " os-maven-plugin extension and depend on arena-junit with classifier"
              + " ${os.detected.classifier} to pull in a platform-native arena_ffi_shared"
              + " library)");
    }
    return lib;
  }

  public static String takeErr(PointerByReference errSlot) {
    return takeOutString(errSlot);
  }

  public static String takeOutString(PointerByReference slot) {
    Pointer p = slot.getValue();
    if (p == null) {
      return null;
    }
    try {
      long peer = Pointer.nativeValue(p);
      if (peer == 0) {
        return null;
      }
      return p.getString(0, StandardCharsets.UTF_8.name());
    } finally {
      ArenaNativeHolder.LIB.arena_free_string(p);
      slot.setValue(null);
    }
  }

  public static Pointer arenaOpen(
      String name, String configJson, ArenaLogLevel logLevel) {
    ArenaNativeLib lib = lib();
    lib.arena_set_log_level(logLevel.code());
    PointerByReference err = new PointerByReference();
    PointerByReference state = new PointerByReference();
    Pointer h = lib.arena_open(name, configJson, err, state);
    String stateDocument = takeOutString(state);
    if (h == null || Pointer.nativeValue(h) == 0) {
      String msg = takeErr(err);
      throw new ArenaBindingError(
          msg != null ? msg : "arena_open returned null", null, stateDocument);
    }
    return h;
  }

  public static void setDispatcherDependencyAllowJson(String jsonUtf8MaybeNull) {
    lib().arena_dispatcher_dependency_allow_json_set(jsonUtf8MaybeNull);
  }

  public static void setDispatcherComponentAllowJson(String jsonUtf8MaybeNull) {
    lib().arena_dispatcher_component_allow_json_set(jsonUtf8MaybeNull);
  }

  public static long registerDefaultDispatcherLoggingTarget() {
    return registerDefaultDispatcherLoggingTarget(ArenaLogLevel.INFO);
  }

  public static long registerDefaultDispatcherLoggingTarget(ArenaLogLevel arenaLogLevel) {
    ArenaPlatformLoggingTarget.installJulDirectStderr(arenaLogLevel);
    long token = lib().arena_add_log_target(ArenaPlatformLoggingTarget.INSTANCE, Pointer.NULL);
    if (token == 0L) {
      ArenaPlatformLoggingTarget.removeJulDirectStderrInstallation();
      throw new ArenaBindingError("arena_add_log_target rejected callback");
    }
    DEFAULT_DISPATCHER_LOGGING_TARGET_TOKENS.add(token);
    return token;
  }

  public static long registerSlf4jDispatcherLoggingTarget(Logger logger) {
    return registerSlf4jDispatcherLoggingTarget(logger, ArenaLogLevel.INFO);
  }

  public static long registerSlf4jDispatcherLoggingTarget(
      Logger logger, ArenaLogLevel arenaLogLevel) {
    ArenaSlf4jDispatcherStderrPublication publication =
        ArenaSlf4jDispatcherStderrPublication.installIfApplicable(logger, arenaLogLevel);
    long token;
    try {
      token = registerDispatcherLoggingTarget(new ArenaSlf4jLoggingTarget(logger), Pointer.NULL);
    } catch (RuntimeException e) {
      if (publication != null) {
        publication.restore();
      }
      throw e;
    }
    if (publication != null) {
      SLF4J_DISPATCHER_STDERR_PUBLICATIONS.put(token, publication);
    }
    return token;
  }

  public static long registerSlf4jDispatcherLoggingTarget(ILoggerFactory loggerFactory) {
    return registerSlf4jDispatcherLoggingTarget(loggerFactory, ArenaLogLevel.INFO);
  }

  public static long registerSlf4jDispatcherLoggingTarget(
      ILoggerFactory loggerFactory, ArenaLogLevel arenaLogLevel) {
    ArenaSlf4jLogbackAlign.alignSlf4jLoggerWithArenaLogLevel(
        loggerFactory.getLogger(ArenaSlf4jLoggingTarget.ROOT_LOGGER_NAME), arenaLogLevel);
    return registerDispatcherLoggingTarget(
        new ArenaSlf4jLoggingTarget(loggerFactory), Pointer.NULL);
  }

  public static long registerDispatcherLoggingTarget(
      ArenaLoggingTargetCallback callback, Pointer userData) {
    if (callback == null) {
      throw new ArenaBindingError("log target callback is null");
    }
    Pointer bound = userData != null ? userData : Pointer.NULL;
    long token = lib().arena_add_log_target(callback, bound);
    if (token == 0L) {
      throw new ArenaBindingError("arena_add_log_target rejected callback");
    }
    return token;
  }

  public static void unregisterDispatcherLoggingTarget(long token) {
    if (token == 0L) {
      return;
    }
    lib().arena_remove_log_target(token);
    if (DEFAULT_DISPATCHER_LOGGING_TARGET_TOKENS.remove(token)) {
      ArenaPlatformLoggingTarget.removeJulDirectStderrInstallation();
      return;
    }
    ArenaSlf4jDispatcherStderrPublication pub =
        SLF4J_DISPATCHER_STDERR_PUBLICATIONS.remove(token);
    if (pub != null) {
      pub.restore();
    }
  }

  public static Pointer arenaOpen(String name, String configJson) {
    return arenaOpen(name, configJson, ArenaLogLevel.INFO);
  }

  public static String arenaClose(Pointer handle) {
    if (handle == null || Pointer.nativeValue(handle) == 0) {
      return null;
    }
    PointerByReference err = new PointerByReference();
    PointerByReference state = new PointerByReference();
    int status = ArenaNativeHolder.LIB.arena_close(handle, err, state);
    String message = takeOutString(err);
    String stateDocument = takeOutString(state);
    if (status != 0) {
      throw new ArenaBindingError(
          message != null ? message : "arena_close (status_code=" + status + ")",
          ArenaStatus.fromInt(status),
          stateDocument);
    }
    return stateDocument;
  }

  public static String arenaStateJson(Pointer handle) {
    if (handle == null || Pointer.nativeValue(handle) == 0) {
      throw new ArenaBindingError("arena_state_json called on closed arena");
    }
    PointerByReference err = new PointerByReference();
    PointerByReference state = new PointerByReference();
    int status = ArenaNativeHolder.LIB.arena_state_json(handle, err, state);
    String message = takeOutString(err);
    String stateDocument = takeOutString(state);
    if (status != 0) {
      throw new ArenaBindingError(
          message != null ? message : "arena_state_json (status_code=" + status + ")",
          ArenaStatus.fromInt(status));
    }
    return stateDocument != null ? stateDocument : "{}";
  }

  private static final Map<Long, ArenaLifecycleObserverCallback> LIFECYCLE_OBSERVERS =
      new java.util.concurrent.ConcurrentHashMap<>();

  public static long addLifecycleObserver(java.util.function.Consumer<String> onStateDocument) {
    if (onStateDocument == null) {
      throw new ArenaBindingError("lifecycle observer consumer is null");
    }
    ArenaLifecycleObserverCallback callback =
        (stateJsonUtf8, ignoredUserData) -> {
          if (stateJsonUtf8 == null) {
            return;
          }
          String document =
              stateJsonUtf8.getString(0, java.nio.charset.StandardCharsets.UTF_8.name());
          if (document != null && !document.isEmpty()) {
            onStateDocument.accept(document);
          }
        };
    long token = lib().arena_add_lifecycle_observer(callback, Pointer.NULL);
    if (token == 0L) {
      throw new ArenaBindingError("arena_add_lifecycle_observer rejected callback");
    }
    LIFECYCLE_OBSERVERS.put(token, callback);
    return token;
  }

  public static void removeLifecycleObserver(long token) {
    if (token == 0L) {
      return;
    }
    ArenaLifecycleObserverCallback callback = LIFECYCLE_OBSERVERS.remove(token);
    if (callback == null) {
      return;
    }
    lib().arena_remove_lifecycle_observer(token);
  }

  public static ArenaStatus softReset(Pointer arena, String dependencyIdentifier) {
    return reset(arena, dependencyIdentifier, true);
  }

  public static ArenaStatus hardReset(Pointer arena, String dependencyIdentifier) {
    return reset(arena, dependencyIdentifier, false);
  }

  private static ArenaStatus reset(Pointer arena, String dependencyIdentifier, boolean soft) {
    PointerByReference err = new PointerByReference();
    int raw =
        soft
            ? ArenaNativeHolder.LIB.arena_soft_reset(arena, dependencyIdentifier, err)
            : ArenaNativeHolder.LIB.arena_hard_reset(arena, dependencyIdentifier, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "reset returned unknown status " + raw);
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "reset failed: " + st, st);
    }
    return st;
  }

  public static int findAvailablePort(int rangeStart, int rangeEnd, PortSearchStrategy strategy) {
    PointerByReference err = new PointerByReference();
    IntByReference portOut = new IntByReference();
    int raw =
        lib().arena_find_available_port(rangeStart, rangeEnd, strategy.code(), portOut, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "find_available_port returned unknown status " + raw);
    }
    if (st == ArenaStatus.PANIC) {
      throw new ArenaPortNotFoundException(msg != null ? msg : "no available port found");
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "find_available_port failed: " + st, st);
    }
    return portOut.getValue();
  }

  public static String oauthLoopbackTlsPemJson() {
    ArenaNativeLib lib = lib();
    PointerByReference err = new PointerByReference();
    Pointer raw = lib.arena_oauth_loopback_tls_pem_json(err);
    if (raw == null || Pointer.nativeValue(raw) == 0) {
      String msg = takeErr(err);
      throw new ArenaBindingError(msg != null ? msg : "arena_oauth_loopback_tls_pem_json returned null");
    }
    try {
      return raw.getString(0, StandardCharsets.UTF_8.name());
    } finally {
      lib.arena_free_string(raw);
    }
  }

  public static String oauthSignClaims(
      Pointer arena, String dependencyIdentifier, String providerJson, String claimsJson) {
    ArenaNativeLib lib = lib();
    PointerByReference err = new PointerByReference();
    Pointer raw = lib.arena_oauth_sign_claims(arena, dependencyIdentifier, providerJson, claimsJson, err);
    if (raw == null || Pointer.nativeValue(raw) == 0) {
      String msg = takeErr(err);
      throw new ArenaBindingError(msg != null ? msg : "arena_oauth_sign_claims returned null");
    }
    try {
      return raw.getString(0, StandardCharsets.UTF_8.name());
    } finally {
      lib.arena_free_string(raw);
    }
  }

  public static Pointer matchPlaybookRun(Pointer arena, String identifier) {
    ArenaNativeLib lib = lib();
    PointerByReference err = new PointerByReference();
    Pointer h = lib.arena_match_playbook_run(arena, identifier, err);
    if (h == null || Pointer.nativeValue(h) == 0) {
      String msg = takeErr(err);
      throw new ArenaBindingError(msg != null ? msg : "arena_match_playbook_run returned null");
    }
    return h;
  }

  public static void activePlaybookDrop(Pointer handle) {
    if (handle == null || Pointer.nativeValue(handle) == 0) {
      return;
    }
    PointerByReference err = new PointerByReference();
    int raw = lib().arena_active_playbook_drop(handle, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "active_playbook_drop unknown status " + raw);
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "active_playbook_drop failed: " + st, st);
    }
  }

  public static Pointer httpPlaybookOpen(Pointer arena, String specJson) {
    ArenaNativeLib lib = lib();
    PointerByReference err = new PointerByReference();
    Pointer h = lib.arena_http_playbook_open(arena, specJson, err);
    if (h == null || Pointer.nativeValue(h) == 0) {
      String msg = takeErr(err);
      throw new ArenaBindingError(msg != null ? msg : "arena_http_playbook_open returned null");
    }
    return h;
  }

  public static void httpPlaybookVerify(Pointer handle, String specJson) {
    PointerByReference err = new PointerByReference();
    int raw = lib().arena_http_playbook_verify(handle, specJson, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "http_playbook_verify unknown status " + raw);
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "http_playbook_verify failed: " + st, st);
    }
  }

  public static void mssqlPlaybookVerify(Pointer handle, String specJson) {
    PointerByReference err = new PointerByReference();
    int raw = lib().arena_mssql_playbook_verify(handle, specJson, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "mssql_playbook_verify unknown status " + raw);
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "mssql_playbook_verify failed: " + st, st);
    }
  }

  public static void postgresPlaybookVerify(Pointer handle, String specJson) {
    PointerByReference err = new PointerByReference();
    int raw = lib().arena_postgres_playbook_verify(handle, specJson, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "postgres_playbook_verify unknown status " + raw);
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "postgres_playbook_verify failed: " + st, st);
    }
  }

  public static void oraclePlaybookVerify(Pointer handle, String specJson) {
    PointerByReference err = new PointerByReference();
    int raw = lib().arena_oracle_playbook_verify(handle, specJson, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "oracle_playbook_verify unknown status " + raw);
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "oracle_playbook_verify failed: " + st, st);
    }
  }
}
