package arena.junit;

public final class LifecycleDashboardBuilder {

  private final ClosedArena closedArena;

  LifecycleDashboardBuilder(ClosedArena closedArena) {
    this.closedArena = closedArena;
  }

  public LifecycleDashboardBuilder withCpuProfiler(String componentId) {
    return new LifecycleDashboardBuilder(closedArena.withCpuProfiledComponent(componentId));
  }

  public LifecycleDashboardBuilder withCpuProfileLimitBytes(long limitBytes) {
    return new LifecycleDashboardBuilder(closedArena.withCpuProfileLimitBytes(limitBytes));
  }

  public ClosedArena done() {
    return closedArena;
  }

  public OpenArena open() throws Exception {
    return closedArena.open();
  }
}
