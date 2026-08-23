using System.Collections.Generic;
using System.Threading.Tasks;
using Microsoft.AspNetCore.Mvc;
using ArenaExamples.Readings.Aspnet.Models;
using ArenaExamples.Readings.Aspnet.Services;

namespace ArenaExamples.Readings.Aspnet.Controllers;

[ApiController]
[Route("[controller]")]
public class WeatherController : ControllerBase
{
    private readonly IWeatherService _weatherService;

    public WeatherController(IWeatherService weatherService)
    {
        _weatherService = weatherService;
    }

    [HttpGet]
    public async Task<ActionResult<List<WeatherReportRow>>> GetAll()
    {
        var reports = await _weatherService.GetAllAsync();
        return Ok(reports);
    }

    [HttpPost]
    public async Task<IActionResult> Create([FromBody] CreateWeatherReportRequest request)
    {
        var response = await _weatherService.CreateAsync(request);
        return Ok(response);
    }
}
