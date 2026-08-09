using System.Threading.Tasks;
using Microsoft.AspNetCore.Mvc;
using ArenaExamples.Readings.Aspnet.Services;

namespace ArenaExamples.Readings.Aspnet.Controllers;

[ApiController]
[Route("[controller]")]
public class HealthController : ControllerBase
{
    private readonly DeviceLifecycleWorkerHostedService _deviceWorker;

    public HealthController(DeviceLifecycleWorkerHostedService deviceWorker)
    {
        _deviceWorker = deviceWorker;
    }

    [HttpGet]
    public IActionResult Get()
    {
        if (!_deviceWorker.IsReady)
            return StatusCode(503, "starting");
        return Ok("ok");
    }
}
