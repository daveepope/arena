using ArenaDotnet.Xunit;
using Xunit;

public class BazelConsumerSmokeTest
{
    [Fact]
    public void OpenAndCloseArena_ImportedViaBazelCsharpImport_Succeeds()
    {
        using var fixture = new BazelConsumerSmokeTestFixture();
        Assert.NotNull(fixture.Arena);
    }
}

sealed class BazelConsumerSmokeTestFixture : ArenaCollectionFixture
{
    protected override Match Configure() => new MatchBuilder("bazel-consumer-smoke-test-match").Build();
}
