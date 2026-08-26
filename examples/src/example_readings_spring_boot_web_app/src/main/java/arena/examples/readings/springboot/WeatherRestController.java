package arena.examples.readings.springboot;

import jakarta.validation.Valid;
import java.util.List;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
@RequestMapping("/weather")
public class WeatherRestController {

  private final WeatherService weather;

  public WeatherRestController(WeatherService weather) {
    this.weather = weather;
  }

  @GetMapping
  public List<WeatherReportRow> list() {
    return weather.listWeatherReports();
  }

  @PostMapping
  public CreateWeatherReportResponse create(@Valid @RequestBody CreateWeatherReportRequest body) {
    return weather.createWeatherReport(body);
  }
}
