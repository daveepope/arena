using Microsoft.AspNetCore.Mvc;

namespace ArenaExamples.Readings.Aspnet.Controllers;

[ApiController]
[Route("[controller]")]
public class HealthController : ControllerBase
{
    [HttpGet]
    public IActionResult Get()
    {
        return Ok("ok");
    }
}
