package arena.examples.readings.springboot;

import arena.examples.readings.springboot.workflow.DeviceLifecycleWorkflow;
import arena.examples.readings.springboot.workflow.DeviceSnapshot;
import arena.examples.readings.springboot.workflow.DeviceWorkflowClientFactory;
import io.temporal.client.WorkflowClient;
import io.temporal.client.WorkflowNotFoundException;
import java.util.List;
import org.springframework.beans.factory.annotation.Qualifier;
import org.springframework.http.HttpStatus;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.stereotype.Service;
import org.springframework.web.server.ResponseStatusException;

@Service
public class DeviceService {

  private static final long TRANSITION_WAIT_TIMEOUT_MILLIS = 500L;
  private static final long TRANSITION_WAIT_POLL_MILLIS = 25L;

  private final JdbcTemplate pg;
  private final DeviceWorkflowClientFactory workflows;
  private final ProvisionedEmailSender provisionedEmailSender;

  public DeviceService(
      @Qualifier("postgresJdbcTemplate") JdbcTemplate pg,
      DeviceWorkflowClientFactory workflows,
      ProvisionedEmailSender provisionedEmailSender) {
    this.pg = pg;
    this.workflows = workflows;
    this.provisionedEmailSender = provisionedEmailSender;
  }

  public List<DeviceRow> listDevices() {
    return pg.query(
        "select id, name from instrument_reading.device order by id",
        (rs, rowNum) -> new DeviceRow(rs.getLong("id"), rs.getString("name")));
  }

  public CreateDeviceResponse createDevice(CreateDeviceRequest req) {
    long id =
        pg.queryForObject(
            "insert into instrument_reading.device(name) values (?) returning id",
            Long.class,
            req.name());
    DeviceLifecycleWorkflow workflow = workflows.newWorkflowStub(id);
    try {
      WorkflowClient.start(workflow::run, id);
    } catch (RuntimeException e) {
      pg.update("delete from instrument_reading.device where id = ?", id);
      throw new ResponseStatusException(
          HttpStatus.BAD_GATEWAY, "failed to start device workflow for device " + id, e);
    }
    try {
      provisionedEmailSender.sendDeviceProvisionedEmail(id, req.name());
    } catch (RuntimeException ignored) {
    }
    return new CreateDeviceResponse(id, req.name());
  }

  public DeviceStateResponse requestStateTransition(long deviceId, SetDeviceStateRequest req)
      throws InterruptedException {
    DeviceLifecycleWorkflow workflow = workflows.existingWorkflowStub(deviceId);
    try {
      int countBefore = workflow.snapshot().transitionCount();
      workflow.requestTransition(req.target());

      long deadline = System.currentTimeMillis() + TRANSITION_WAIT_TIMEOUT_MILLIS;
      DeviceSnapshot snapshot = workflow.snapshot();
      while (snapshot.transitionCount() == countBefore && System.currentTimeMillis() < deadline) {
        Thread.sleep(TRANSITION_WAIT_POLL_MILLIS);
        snapshot = workflow.snapshot();
      }

      return new DeviceStateResponse(deviceId, snapshot.state(), snapshot.transitionCount());
    } catch (WorkflowNotFoundException e) {
      throw new ResponseStatusException(HttpStatus.NOT_FOUND, "device not found: " + deviceId, e);
    }
  }

  public DeviceStateResponse currentState(long deviceId) {
    DeviceLifecycleWorkflow workflow = workflows.existingWorkflowStub(deviceId);
    try {
      DeviceSnapshot snapshot = workflow.snapshot();
      return new DeviceStateResponse(deviceId, snapshot.state(), snapshot.transitionCount());
    } catch (WorkflowNotFoundException e) {
      throw new ResponseStatusException(HttpStatus.NOT_FOUND, "device not found: " + deviceId, e);
    }
  }

  public void stopDevice(long deviceId) {
    DeviceLifecycleWorkflow workflow = workflows.existingWorkflowStub(deviceId);
    try {
      workflow.stop();
    } catch (WorkflowNotFoundException e) {
      throw new ResponseStatusException(HttpStatus.NOT_FOUND, "device not found: " + deviceId, e);
    }
  }
}
