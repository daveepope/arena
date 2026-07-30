using Amazon;
using Amazon.EventBridge;
using Amazon.EventBridge.Model;
using System.Text.Json;

namespace ArenaExamples.Readings.Aspnet.Services;

public interface IEventBridgePublisher
{
    Task PublishAsync(string eventType, object detail);
}

public class EventBridgePublisher : IEventBridgePublisher
{
    private readonly IAmazonEventBridge _client;
    private readonly string _busName;
    private readonly string _source;

    public EventBridgePublisher(string awsEndpointUrl, string busName, string source)
    {
        _busName = busName;
        _source = source;

        var config = new AmazonEventBridgeConfig
        {
            ServiceURL = awsEndpointUrl,
            ForcePathStyle = true
        };
        _client = new AmazonEventBridgeClient("test", "test", config);
    }

    public async Task PublishAsync(string eventType, object detail)
    {
        try
        {
            var request = new PutEventsRequest
            {
                Entries = new List<PutEventsRequestEntry>
                {
                    new()
                    {
                        DetailType = eventType,
                        Detail = JsonSerializer.Serialize(detail),
                        EventBusName = _busName,
                        Source = _source
                    }
                }
            };
            await _client.PutEventsAsync(request);
        }
        catch
        {
        }
    }
}
