using System;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class PlaybookAttributeTest
{
    [Fact]
    public void Constructor_ValidType_SetsPlaybookType()
    {
        var attr = new PlaybookAttribute(typeof(string));
        Assert.Equal(typeof(string), attr.PlaybookType);
    }

    [Fact]
    public void Constructor_NullType_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => new PlaybookAttribute(null!));
    }
}
