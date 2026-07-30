using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class HttpPlaybookBuilderPutDeleteTest
{
    [Fact]
    public void put_mapping_sets_method_to_put()
    {
        var builder = new Playbook.HttpPlaybookBuilder("dep-id")
            .Put("/resource")
            .ExpectCalled(1);
        var mappings = builder.BuildMappings();
        Assert.Single(mappings);
        var method = mappings[0].GetType().GetProperty("Method")?.GetValue(mappings[0]);
        Assert.Equal("PUT", method);
    }

    [Fact]
    public void delete_mapping_sets_method_to_delete()
    {
        var builder = new Playbook.HttpPlaybookBuilder("dep-id")
            .Delete("/resource/1")
            .ExpectCalled(1);
        var mappings = builder.BuildMappings();
        Assert.Single(mappings);
        var method = mappings[0].GetType().GetProperty("Method")?.GetValue(mappings[0]);
        Assert.Equal("DELETE", method);
    }

    [Fact]
    public void then_return_adds_response_to_list()
    {
        var builder = new Playbook.HttpPlaybookBuilder("dep-id")
            .Get("/api")
            .WillReturn(Playbook.HttpResponse.Ok())
            .ThenReturn(Playbook.HttpResponse.Created())
            .ExpectCalled(1);
        var mappings = builder.BuildMappings();
        var responses = mappings[0].GetType().GetProperty("Responses")?.GetValue(mappings[0]) as System.Collections.IList;
        Assert.NotNull(responses);
        Assert.Equal(2, responses.Count);
    }

    [Fact]
    public void duplicate_mapping_does_not_add_duplicate()
    {
        var m2 = new Playbook.HttpPlaybookBuilder("dep-id")
            .Get("/api")
            .ExpectCalled(2);
        var mappings = m2.BuildMappings();
        Assert.Single(mappings);
    }
}
