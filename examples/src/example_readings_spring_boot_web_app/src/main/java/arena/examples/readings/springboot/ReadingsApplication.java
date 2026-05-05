package arena.examples.readings.springboot;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;

@SpringBootApplication
public class ReadingsApplication {

  public static void main(String[] args) {
    String wp = System.getenv("WEB_APP_PORT");
    if (wp != null && !wp.isBlank()) {
      System.setProperty("server.port", wp.trim());
    }
    SpringApplication.run(ReadingsApplication.class, args);
  }
}
