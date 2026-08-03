using System;
using ArenaXunit.Ffi;
using ArenaXunit.Playbook;
using Xunit;

namespace ArenaXunit.UnitTest;

public class ActivePlaybookTest
{
    [Fact]
    public void Dispose_CalledMultipleTimes_DoesNotThrow()
    {
        var playbook = new ActiveLocalstackPlaybook(IntPtr.Zero);
        playbook.Dispose();
        playbook.Dispose();
    }

    [Fact]
    public void Verify_NullHandle_Mssql_ThrowsArenaBindingError()
    {
        using var playbook = new ActiveMssqlPlaybook(IntPtr.Zero);
        Assert.Throws<ArenaBindingError>(() => playbook.Verify("SELECT 1", 1));
    }

    [Fact]
    public void Verify_NullHandle_Http_ThrowsArenaBindingError()
    {
        using var playbook = new ActiveHttpPlaybook(IntPtr.Zero);
        Assert.Throws<ArenaBindingError>(() => playbook.Verify("GET", "/x", 1));
    }

    [Fact]
    public void VerifyAtLeast_NullHandle_Http_ThrowsArenaBindingError()
    {
        using var playbook = new ActiveHttpPlaybook(IntPtr.Zero);
        Assert.Throws<ArenaBindingError>(() => { playbook.VerifyAtLeast("GET", "/x", 1); });
    }

    [Fact]
    public void WrapHandle_HttpPlaybook_ReturnsActiveHttpPlaybook()
    {
        var playbook = new TestManagedHttpPlaybook("id", "dep-id");
        using var active = playbook.WrapHandle(IntPtr.Zero);
        Assert.IsType<ActiveHttpPlaybook>(active);
    }

    [Fact]
    public void WrapHandle_MssqlPlaybook_ReturnsActiveMssqlPlaybook()
    {
        var playbook = new TestManagedMssqlPlaybook("id", "dep-id");
        using var active = playbook.WrapHandle(IntPtr.Zero);
        Assert.IsType<ActiveMssqlPlaybook>(active);
    }

    [Fact]
    public void WrapHandle_LocalstackPlaybook_ReturnsActiveLocalstackPlaybook()
    {
        var playbook = new TestManagedLocalstackPlaybook("id", "dep-id");
        using var active = playbook.WrapHandle(IntPtr.Zero);
        Assert.IsType<ActiveLocalstackPlaybook>(active);
    }

    private class TestManagedHttpPlaybook : ManagedHttpPlaybook
    {
        public TestManagedHttpPlaybook(string id, string depId)
            : base(id, depId, new System.Collections.Generic.List<object>()) { }
    }

    private class TestManagedMssqlPlaybook : ManagedMssqlPlaybook
    {
        public TestManagedMssqlPlaybook(string id, string depId) : base(id, depId) { }
    }

    private class TestManagedLocalstackPlaybook : ManagedLocalstackPlaybook
    {
        public TestManagedLocalstackPlaybook(string id, string depId) : base(id, depId) { }
    }
}
