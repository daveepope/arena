package arena.examples.readings.springboot;

import org.springframework.http.MediaType;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
public class HealthRestController {

  @GetMapping(value = "/health", produces = MediaType.TEXT_PLAIN_VALUE)
  public String health() {
    return "ok";
  }
}
