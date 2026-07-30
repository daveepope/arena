using System.Linq;
using ArenaXunit.Playbook;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class PlaybookBuilderSerializationTest
{
    [Fact]
    public void http_playbook_build_post_mapping_serializes_correct_json()
    {
        var builder = new HttpPlaybookBuilder("dep-id")
            .Post("/api/validate")
            .ExpectCalled(1);
        var mappings = builder.BuildMappings();
        Assert.Single(mappings);
        var mapping = mappings[0];
        var method = mapping.GetType().GetProperty("Method")?.GetValue(mapping);
        Assert.Equal("POST", method);
    }

    [Fact]
    public void http_playbook_build_get_mapping_serializes_correct_json()
    {
        var builder = new HttpPlaybookBuilder("dep-id")
            .Get("/health")
            .ExpectCalled(1);
        var mappings = builder.BuildMappings();
        var method = mappings[0].GetType().GetProperty("Method")?.GetValue(mappings[0]);
        Assert.Equal("GET", method);
    }

    [Fact]
    public void http_playbook_build_with_response_serializes_status()
    {
        var builder = new HttpPlaybookBuilder("dep-id")
            .Post("/api/validate")
            .WillReturn(HttpResponse.OkJson(new { valid = true }))
            .ExpectCalled(1);
        var mappings = builder.BuildMappings();
        var responses = mappings[0].GetType().GetProperty("Responses")?.GetValue(mappings[0]) as System.Collections.IList;
        Assert.NotNull(responses);
        Assert.Single(responses);
        var response = responses[0];
        var status = response.GetType().GetProperty("Status")?.GetValue(response);
        Assert.Equal(200, status);
    }

    [Fact]
    public void http_playbook_build_expect_never_called_sets_flag()
    {
        var builder = new HttpPlaybookBuilder("dep-id")
            .Get("/never")
            .ExpectNeverCalled();
        var mappings = builder.BuildMappings();
        var expect = mappings[0].GetType().GetProperty("Expect")?.GetValue(mappings[0]);
        Assert.NotNull(expect);
        var neverCalled = expect.GetType().GetProperty("NeverCalled")?.GetValue(expect);
        Assert.True(neverCalled);
    }
}
