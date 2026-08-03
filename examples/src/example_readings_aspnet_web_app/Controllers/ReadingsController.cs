using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;
using ArenaExamples.Readings.Aspnet.Models;
using ArenaExamples.Readings.Aspnet.Services;

namespace ArenaExamples.Readings.Aspnet.Controllers;

[ApiController]
[Route("[controller]")]
public class ReadingsController : ControllerBase
{
    private readonly IReadingsService _readingsService;
    private readonly ILogger<ReadingsController> _logger;

    public ReadingsController(IReadingsService readingsService, ILogger<ReadingsController> logger)
    {
        _readingsService = readingsService;
        _logger = logger;
    }

    [HttpGet]
    public async Task<ActionResult<List<ReadingRow>>> GetAll()
    {
        var readings = await _readingsService.GetAllAsync();
        return Ok(readings);
    }

    [HttpPost]
    public async Task<IActionResult> Create([FromBody] CreateReadingRequest request)
    {
        if (string.IsNullOrEmpty(request.UserName))
            return BadRequest();

        try
        {
            var response = await _readingsService.CreateAsync(request);
            return Ok(response);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to create reading");
            return StatusCode(500, new { error = ex.Message });
        }
    }
}
