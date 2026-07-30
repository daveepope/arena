using ArenaXunit.Playbook;
using Xunit;

namespace ArenaXunit.UnitTest;

public class HttpResponseTest
{
    [Fact]
    public void ok_returns_status_200()
    {
        var response = HttpResponse.Ok();
        Assert.Equal(200, response.Status);
    }

    [Fact]
    public void ok_json_returns_status_200_with_json_content_type()
    {
        var response = HttpResponse.OkJson(new { valid = true });
        Assert.Equal(200, response.Status);
        Assert.Equal("application/json", response.ContentType);
    }

    [Fact]
    public void created_returns_status_201()
    {
        var response = HttpResponse.Created();
        Assert.Equal(201, response.Status);
    }

    [Fact]
    public void no_content_returns_status_204()
    {
        var response = HttpResponse.NoContent();
        Assert.Equal(204, response.Status);
    }

    [Fact]
    public void server_error_returns_status_500()
    {
        var response = HttpResponse.ServerError();
        Assert.Equal(500, response.Status);
    }

    [Fact]
    public void status_with_code_returns_correct_status()
    {
        var response = HttpResponse.Status(418);
        Assert.Equal(418, response.Status);
    }

    [Fact]
    public void status_with_code_and_body_returns_correct_values()
    {
        var response = HttpResponse.Status(200, "hello");
        Assert.Equal(200, response.Status);
        Assert.Equal("hello", response.Body);
        Assert.Equal("text/plain", response.ContentType);
    }

    [Fact]
    public void status_json_returns_correct_content_type()
    {
        var response = HttpResponse.StatusJson(201, new { id = 1 });
        Assert.Equal(201, response.Status);
        Assert.Equal("application/json", response.ContentType);
    }
}
