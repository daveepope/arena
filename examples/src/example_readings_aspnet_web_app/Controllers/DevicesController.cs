using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Microsoft.AspNetCore.Mvc;
using ArenaExamples.Readings.Aspnet.Models;
using ArenaExamples.Readings.Aspnet.Services;

namespace ArenaExamples.Readings.Aspnet.Controllers;

[ApiController]
[Route("[controller]")]
public class DevicesController : ControllerBase
{
    private readonly IDevicesService _devicesService;

    public DevicesController(IDevicesService devicesService)
    {
        _devicesService = devicesService;
    }

    [HttpGet]
    public async Task<ActionResult<List<DeviceRow>>> GetAll()
    {
        var devices = await _devicesService.GetAllAsync();
        return Ok(devices);
    }

    [HttpPost]
    public async Task<IActionResult> Create([FromBody] CreateDeviceRequest request)
    {
        if (string.IsNullOrEmpty(request.Name))
            return BadRequest();

        try
        {
            var response = await _devicesService.CreateAsync(request);
            return Ok(response);
        }
        catch (Exception ex)
        {
            return StatusCode(502, new { error = ex.Message });
        }
    }

    [HttpGet("{deviceId}/state")]
    public async Task<IActionResult> GetState(int deviceId)
    {
        var state = await _devicesService.GetStateAsync(deviceId);
        if (state == null)
            return NotFound();
        return Ok(state);
    }

    [HttpPost("{deviceId}/state")]
    public async Task<IActionResult> SetState(int deviceId, [FromBody] SetDeviceStateRequest request)
    {
        var state = await _devicesService.SetStateAsync(deviceId, request.Target);
        if (state == null)
            return NotFound();
        return Ok(state);
    }

    [HttpDelete("{deviceId}")]
    public async Task<IActionResult> Delete(int deviceId)
    {
        var stopped = await _devicesService.DeleteAsync(deviceId);
        if (!stopped)
            return NotFound();
        return Ok();
    }
}
