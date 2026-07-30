using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Microsoft.AspNetCore.Mvc;
using ArenaExamples.Readings.Aspnet.Models;
using ArenaExamples.Readings.Aspnet.Services;

namespace ArenaExamples.Readings.Aspnet.Controllers;

[ApiController]
[Route("[controller]")]
public class ReadingsController : ControllerBase
{
    private readonly IReadingsService _readingsService;

    public ReadingsController(IReadingsService readingsService)
    {
        _readingsService = readingsService;
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
            return StatusCode(500, new { error = ex.Message });
        }
    }
}
