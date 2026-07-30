using ArenaXunit.Dep;
using ArenaXunit.Topology;
using Xunit;

namespace ArenaXunit.UnitTest;

public class RegisteredPlaybookTest
{
    [Fact]
    public void ToConfig_HttpPlaybook_ReturnsHttpKind()
    {
        var playbook = new TestManagedHttpPlaybook("id", "dep-id");
        var rp = new RegisteredPlaybook(playbook, false);
        var config = rp.ToConfig();
        var kind = config.GetType().GetProperty("kind")?.GetValue(config);
        Assert.Equal("http", kind);
    }

    [Fact]
    public void ToConfig_MssqlPlaybook_ReturnsMssqlKind()
    {
        var playbook = new TestManagedMssqlPlaybook("id", "dep-id");
        var rp = new RegisteredPlaybook(playbook, true);
        var config = rp.ToConfig();
        var kind = config.GetType().GetProperty("kind")?.GetValue(config);
        Assert.Equal("mssql", kind);
    }

    [Fact]
    public void ToConfig_LocalstackPlaybook_ReturnsLocalstackKind()
    {
        var playbook = new TestManagedLocalstackPlaybook("id", "dep-id");
        var rp = new RegisteredPlaybook(playbook, false);
        var config = rp.ToConfig();
        var kind = config.GetType().GetProperty("kind")?.GetValue(config);
        Assert.Equal("localstack", kind);
    }

    [Fact]
    public void ToConfig_ExecOnDependencyStart_Preserved()
    {
        var playbook = new TestManagedHttpPlaybook("id", "dep-id");
        var rp = new RegisteredPlaybook(playbook, true);
        var config = rp.ToConfig();
        var val = config.GetType().GetProperty("exec_on_dependency_start")?.GetValue(config);
        Assert.Equal(true, val);
    }

    private class TestManagedHttpPlaybook : Playbook.ManagedHttpPlaybook
    {
        public TestManagedHttpPlaybook(string id, string depId)
            : base(id, depId, new System.Collections.Generic.List<object>()) { }
    }

    private class TestManagedMssqlPlaybook : Playbook.ManagedMssqlPlaybook
    {
        public TestManagedMssqlPlaybook(string id, string depId)
            : base(id, depId) { }
    }

    private class TestManagedLocalstackPlaybook : Playbook.ManagedLocalstackPlaybook
    {
        public TestManagedLocalstackPlaybook(string id, string depId)
            : base(id, depId) { }
    }
}
