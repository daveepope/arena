namespace ArenaDotnet.Xunit;

public sealed class LifecycleDashboardBuilder
{
    private readonly ClosedArena _closedArena;

    internal LifecycleDashboardBuilder(ClosedArena closedArena)
    {
        _closedArena = closedArena;
    }

    public LifecycleDashboardBuilder WithCpuProfiler(string componentId) =>
        new(_closedArena.WithCpuProfiledComponent(componentId));

    public LifecycleDashboardBuilder WithCpuProfileLimitBytes(long limitBytes) =>
        new(_closedArena.WithCpuProfileLimitBytes(limitBytes));

    public ClosedArena Done() => _closedArena;

    public System.Threading.Tasks.Task<OpenArena> OpenAsync() => _closedArena.OpenAsync();
}
