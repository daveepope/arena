using ArenaDotnet.Xunit.Playbook;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class UnmanagedPlaybookTest
{
    [Fact]
    public void Identifier_ConcreteSubclass_ReturnsConfiguredValue()
    {
        var playbook = new TestUnmanagedPlaybook("seed-id");
        Assert.Equal("seed-id", playbook.Identifier);
    }

    [Fact]
    public void Run_ConcreteSubclass_ReturnsWrappedActivePlaybook()
    {
        var playbook = new TestUnmanagedPlaybook("seed-id");
        using var active = playbook.Run(null!);
        Assert.IsType<ActiveLocalstackPlaybook>(active);
    }

    internal class TestUnmanagedPlaybook : UnmanagedPlaybook
    {
        public override string Identifier { get; }

        public TestUnmanagedPlaybook(string identifier)
        {
            Identifier = identifier;
        }

        public override ActivePlaybook Run(OpenArena arena)
        {
            return new ActiveLocalstackPlaybook(System.IntPtr.Zero);
        }
    }
}
