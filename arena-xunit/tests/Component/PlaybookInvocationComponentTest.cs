using System;
using System.Net;
using System.Net.Http;
using System.Text;
using System.Threading.Tasks;
using ArenaDotnet.Xunit;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Playbook;
using ArenaDotnet.Xunit.Xunit;
using Xunit;

[assembly: PlaybookExecutionAttribute]

namespace ArenaDotnet.Xunit.ComponentTest;

public class PlaybookInvocationComponentTest : IClassFixture<PlaybookInvocationComponentTest.Fixture>
{
    private static readonly int _httpPort = TestRuntime.AllocatePort();

    private static OpenArena Arena { get; set; } = null!;

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

    private class UnmetExpectationPlaybook : ManagedHttpPlaybook
    {
        public UnmetExpectationPlaybook(string depId)
            : base("playbook-invocation-unmet-expectation", depId,
                new HttpPlaybookBuilder(depId)
                    .Post("/api/v1/validate")
                    .WillReturn(HttpResponse.OkJson(new { valid = true }))
                    .ExpectCalled(1)
                    .BuildMappings())
        {
        }
    }

    public class Fixture : ArenaCollectionFixture
    {
        protected override Match Configure()
        {
            var httpDep = new HttpDependencyBuilder("playbook-invocation-http")
                .WithPort(_httpPort)
                .Build();

            return new MatchBuilder("playbook-invocation-match")
                .AddDependency(httpDep)
                .RegisterPlaybook(new CalibrationHappyPathPlaybook(httpDep.Identifier), true)
                .RegisterPlaybook(new ScopedOutagePlaybook(httpDep.Identifier), false)
                .RegisterPlaybook(new VerifyPlaybook(httpDep.Identifier), false)
                .RegisterPlaybook(new UnmetExpectationPlaybook(httpDep.Identifier), false)
                .Build();
        }
    }

    public PlaybookInvocationComponentTest(Fixture fixture)
    {
        Arena = fixture.Arena;
    }

    [Fact]
    public void SessionDefaultPlaybook_RunsAtArenaOpen()
    {
        var pb = Arena.GetPlaybook(typeof(CalibrationHappyPathPlaybook));
        Assert.NotNull(pb);
        Assert.True(Arena.PlaybookExecOnDependencyStart(typeof(CalibrationHappyPathPlaybook)));
    }

    [Fact]
    public void ScopedPlaybook_RegisteredButNotExecOnStart()
    {
        var pb = Arena.GetPlaybook(typeof(ScopedOutagePlaybook));
        Assert.NotNull(pb);
        Assert.False(Arena.PlaybookExecOnDependencyStart(typeof(ScopedOutagePlaybook)));
    }

    [Fact]
    [Playbook(typeof(ScopedOutagePlaybook))]
    public async Task ScopedPlaybook_ManualRun_ActivatesAndOverrides()
    {
        var response = await PostValidateAsync();
        Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
    }

    [Fact]
    public async Task ScopedPlaybook_AfterDispose_ReturnsToSessionDefault()
    {
        var pb = Arena.GetPlaybook(typeof(ScopedOutagePlaybook));
        Assert.NotNull(pb);
        using (var active = pb.Run(Arena))
        {
            var response = await PostValidateAsync();
            Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
        }
        var response2 = await PostValidateAsync();
        Assert.Equal(HttpStatusCode.OK, response2.StatusCode);
    }

    [Fact]
    [Playbook(typeof(VerifyPlaybook))]
    public async Task VerifyAtLeast_WithTraffic_Succeeds()
    {
        var response = await PostValidateAsync();
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        PlaybookScope.GetActive<ActiveHttpPlaybook>().Verify("POST", "/api/v1/validate", 1);
    }

    [Fact]
    [Playbook(typeof(VerifyPlaybook))]
    public void VerifyAtLeast_WithoutTraffic_Throws()
    {
        Assert.Throws<ArenaDotnet.Xunit.Ffi.ArenaBindingError>(
            () => PlaybookScope.GetActive<ActiveHttpPlaybook>().VerifyAtLeast("POST", "/api/v1/validate", 1));
    }

    [Fact]
    [Playbook(typeof(VerifyPlaybook))]
    public void Verify_Failure_DoesNotThrowDuringScopeTeardown()
    {
        Assert.Throws<ArenaDotnet.Xunit.Ffi.ArenaBindingError>(
            () => PlaybookScope.GetActive<ActiveHttpPlaybook>().Verify("POST", "/api/v1/validate", 1));
    }

    [Fact]
    public void ScopedPlaybook_UnmetExpectCalled_ThrowsOnDispose()
    {
        var pb = Arena.GetPlaybook(typeof(UnmetExpectationPlaybook));
        Assert.NotNull(pb);
        var active = pb!.Run(Arena);
        Assert.Throws<ArenaDotnet.Xunit.Ffi.ArenaBindingError>(() => active.Dispose());
    }

    private static async Task<HttpResponseMessage> PostValidateAsync()
    {
        using var client = new HttpClient();
        var content = new StringContent("{}", Encoding.UTF8, "application/json");
        return await client.PostAsync($"http://127.0.0.1:{_httpPort}/api/v1/validate", content);
    }
}
