package dev.arena.examples.readings.springboot;

import jakarta.validation.Valid;
import java.util.List;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
@RequestMapping("/readings")
public class ReadingsRestController {

  private final ReadingsService readings;

  public ReadingsRestController(ReadingsService readings) {
    this.readings = readings;
  }

  @GetMapping
  public List<ReadingRow> list() {
    return readings.listReadings();
  }

  @PostMapping
  public CreateReadingResponse create(@Valid @RequestBody CreateReadingRequest body) throws Exception {
    return readings.createReading(body);
  }
}
