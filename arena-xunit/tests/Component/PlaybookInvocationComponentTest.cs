using System;
using System.Net;
using System.Net.Http;
using System.Text;
using System.Threading.Tasks;
using ArenaXunit;
using ArenaXunit.Dep;
using ArenaXunit.Playbook;
using ArenaXunit.Topology;
using ArenaXunit.Xunit;
using Xunit;

namespace ArenaXunit.ComponentTest;

public class PlaybookInvocationComponentTest : IClassFixture<PlaybookInvocationComponentTest.Fixture>
{
    private static readonly int _httpPort = TestRuntime.AllocatePort();

    private readonly OpenArena _arena;

    private class CalibrationHappyPathPlaybook : ManagedHttpPlaybook
    {
        public CalibrationHappyPathPlaybook(string depId)
            : base("playbook-invocation-session-default", depId,
                new HttpPlaybookBuilder(depId)
                    .Post("/api/v1/validate")
                    .WillReturn(HttpResponse.OkJson(new { valid = true }))
                    .BuildMappings())
        {
        }
    }

    private class ScopedOutagePlaybook : ManagedHttpPlaybook
    {
        public ScopedOutagePlaybook(string depId)
            : base("playbook-invocation-scoped-outage", depId,
                new HttpPlaybookBuilder(depId)
                    .Post("/api/v1/validate")
                    .WillReturn(HttpResponse.Status(500))
                    .BuildMappings())
        {
        }
    }

    private class VerifyPlaybook : ManagedHttpPlaybook
    {
        public VerifyPlaybook(string depId)
            : base("playbook-invocation-verify", depId,
                new HttpPlaybookBuilder(depId)
                    .Post("/api/v1/validate")
                    .WillReturn(HttpResponse.OkJson(new { valid = true }))
                    .BuildMappings())
        {
        }
    }

    public class Fixture : ArenaCollectionFixture<TestTopology>
    {
    }

    public class TestTopology : IArenaTopology
    {
        public Match Configure()
        {
            var httpDep = new HttpDependencyBuilder("playbook-invocation-http")
                .WithPort(_httpPort)
                .Build();

            return new MatchBuilder("playbook-invocation-match")
                .AddDependency(httpDep)
                .RegisterPlaybook(new CalibrationHappyPathPlaybook(httpDep.Identifier), true)
                .RegisterPlaybook(new ScopedOutagePlaybook(httpDep.Identifier), false)
                .RegisterPlaybook(new VerifyPlaybook(httpDep.Identifier), false)
                .Build();
        }
    }

    public PlaybookInvocationComponentTest(Fixture fixture)
    {
        _arena = fixture.Arena;
    }

    [Fact]
    public void sessionDefaultPlaybook_runsAtArenaOpen()
    {
        var pb = _arena.GetPlaybook(typeof(CalibrationHappyPathPlaybook));
        Assert.NotNull(pb);
        Assert.True(_arena.PlaybookExecOnDependencyStart(typeof(CalibrationHappyPathPlaybook)));
    }

    [Fact]
    public void scopedPlaybook_registeredButNotExecOnStart()
    {
        var pb = _arena.GetPlaybook(typeof(ScopedOutagePlaybook));
        Assert.NotNull(pb);
        Assert.False(_arena.PlaybookExecOnDependencyStart(typeof(ScopedOutagePlaybook)));
    }

    [Fact]
    public async Task scopedPlaybook_manualRun_activatesAndOverrides()
    {
        var pb = _arena.GetPlaybook(typeof(ScopedOutagePlaybook));
        Assert.NotNull(pb);
        using (var active = pb.Run(_arena))
        {
            var response = await PostValidateAsync();
            Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
        }
    }

    [Fact]
    public async Task scopedPlaybook_afterDispose_returnsToSessionDefault()
    {
        var pb = _arena.GetPlaybook(typeof(ScopedOutagePlaybook));
        Assert.NotNull(pb);
        using (var active = pb.Run(_arena))
        {
            var response = await PostValidateAsync();
            Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
        }
        var response2 = await PostValidateAsync();
        Assert.Equal(HttpStatusCode.OK, response2.StatusCode);
    }

    [Fact]
    public async Task verifyAtLeast_withTraffic_succeeds()
    {
        var pb = _arena.GetPlaybook(typeof(VerifyPlaybook));
        Assert.NotNull(pb);
        using (var active = (ActiveHttpPlaybook)pb.Run(_arena))
        {
            var response = await PostValidateAsync();
            Assert.Equal(HttpStatusCode.OK, response.StatusCode);
            active.Verify("POST", "/api/v1/validate", 1);
        }
    }

    [Fact]
    public async Task verifyAtLeast_withoutTraffic_throws()
    {
        var pb = _arena.GetPlaybook(typeof(VerifyPlaybook));
        Assert.NotNull(pb);
        using (var active = (ActiveHttpPlaybook)pb.Run(_arena))
        {
            Assert.Throws<ArenaXunit.Ffi.ArenaBindingError>(() => active.VerifyAtLeast("POST", "/api/v1/validate", 1));
        }
    }

    [Fact]
    public async Task verify_failure_closeDoesNotThrow()
    {
        var pb = _arena.GetPlaybook(typeof(VerifyPlaybook));
        Assert.NotNull(pb);
        var active = (ActiveHttpPlaybook)pb.Run(_arena);
        Assert.Throws<ArenaXunit.Ffi.ArenaBindingError>(() => active.Verify("POST", "/api/v1/validate", 1));
        active.Dispose();
    }

    private static async Task<HttpResponseMessage> PostValidateAsync()
    {
        using var client = new HttpClient();
        var content = new StringContent("{}", Encoding.UTF8, "application/json");
        return await client.PostAsync($"http://127.0.0.1:{_httpPort}/api/v1/validate", content);
    }
}
