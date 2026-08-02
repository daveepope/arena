using ArenaXunit.Playbook;
using Xunit;

namespace ArenaXunit.UnitTest;

public class HttpResponseTest
{
    [Fact]
    public void Ok_ReturnsStatus200()
    {
        var response = HttpResponse.Ok();
        Assert.Equal(200, response.Status);
    }

    [Fact]
    public void OkJson_ReturnsStatus200_WithJsonBody()
    {
        var response = HttpResponse.OkJson(new { valid = true });
        Assert.Equal(200, response.Status);
        Assert.NotNull(response.JsonBody);
    }

    [Fact]
    public void Created_ReturnsStatus201()
    {
        var response = HttpResponse.Created();
        Assert.Equal(201, response.Status);
    }

    [Fact]
    public void NoContent_ReturnsStatus204()
    {
        var response = HttpResponse.NoContent();
        Assert.Equal(204, response.Status);
    }

    [Fact]
    public void ServerError_ReturnsStatus500()
    {
        var response = HttpResponse.ServerError();
        Assert.Equal(500, response.Status);
    }

    [Fact]
    public void Status_WithCode_ReturnsCorrectStatus()
    {
        var response = HttpResponse.Status(418);
        Assert.Equal(418, response.Status);
    }

    [Fact]
    public void Status_WithCodeAndBody_ReturnsCorrectValues()
    {
        var response = HttpResponse.Status(200, "hello");
        Assert.Equal(200, response.Status);
        Assert.Equal("hello", response.JsonBody);
    }

    [Fact]
    public void StatusJson_ReturnsCorrectJsonBody()
    {
        var response = HttpResponse.StatusJson(201, new { id = 1 });
        Assert.Equal(201, response.Status);
        Assert.NotNull(response.JsonBody);
    }
}
