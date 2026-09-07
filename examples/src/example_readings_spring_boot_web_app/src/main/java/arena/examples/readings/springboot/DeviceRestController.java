package arena.examples.readings.springboot;

import jakarta.validation.Valid;
import java.util.List;
import org.springframework.web.bind.annotation.DeleteMapping;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
@RequestMapping("/devices")
public class DeviceRestController {

  private final DeviceService devices;

  public DeviceRestController(DeviceService devices) {
    this.devices = devices;
  }

  @GetMapping
  public List<DeviceRow> list() {
    return devices.listDevices();
  }

  @PostMapping
  public CreateDeviceResponse create(@Valid @RequestBody CreateDeviceRequest body) {
    return devices.createDevice(body);
  }

  @PostMapping("/{id}/state")
  public DeviceStateResponse setState(
      @PathVariable long id, @Valid @RequestBody SetDeviceStateRequest body) {
    return devices.requestStateTransition(id, body);
  }

  @GetMapping("/{id}/state")
  public DeviceStateResponse getState(@PathVariable long id) {
    return devices.currentState(id);
  }

  @DeleteMapping("/{id}")
  public void stop(@PathVariable long id) {
    devices.stopDevice(id);
  }
}
