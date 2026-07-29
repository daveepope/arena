package arena.examples.readings.springboot;

import java.util.Properties;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.mail.SimpleMailMessage;
import org.springframework.mail.javamail.JavaMailSenderImpl;
import org.springframework.stereotype.Component;

@Component
public class ProvisionedEmailSender {

  private final String smtpHost;
  private final int smtpPort;

  public ProvisionedEmailSender(
      @Value("${SMTP_HOST}") String smtpHost, @Value("${SMTP_PORT}") int smtpPort) {
    this.smtpHost = smtpHost;
    this.smtpPort = smtpPort;
  }

  public void sendDeviceProvisionedEmail(long deviceId, String deviceName) {
    JavaMailSenderImpl sender = new JavaMailSenderImpl();
    sender.setHost(smtpHost);
    sender.setPort(smtpPort);
    Properties props = sender.getJavaMailProperties();
    props.put("mail.smtp.starttls.enable", "true");
    props.put("mail.smtp.starttls.required", "true");
    props.put("mail.smtp.ssl.trust", "*");

    SimpleMailMessage message = new SimpleMailMessage();
    message.setFrom("no-reply@arena.example");
    message.setTo("operations@arena.example");
    message.setSubject("Device provisioned: " + deviceName + " (" + deviceId + ")");
    message.setText("Device " + deviceName + " (id=" + deviceId + ") has been provisioned.");
    sender.send(message);
  }
}
