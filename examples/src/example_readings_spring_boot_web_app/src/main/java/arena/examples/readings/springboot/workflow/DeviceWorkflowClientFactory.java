package arena.examples.readings.springboot.workflow;

import io.temporal.client.WorkflowClient;
import io.temporal.client.WorkflowOptions;
import org.springframework.stereotype.Component;

@Component
public class DeviceWorkflowClientFactory {

  private final WorkflowClient client;

  public DeviceWorkflowClientFactory(WorkflowClient client) {
    this.client = client;
  }

  public DeviceLifecycleWorkflow newWorkflowStub(long deviceId) {
    WorkflowOptions options =
        WorkflowOptions.newBuilder()
            .setTaskQueue(TemporalConfiguration.taskQueue())
            .setWorkflowId(workflowId(deviceId))
            .build();
    return client.newWorkflowStub(DeviceLifecycleWorkflow.class, options);
  }

  public DeviceLifecycleWorkflow existingWorkflowStub(long deviceId) {
    return client.newWorkflowStub(DeviceLifecycleWorkflow.class, workflowId(deviceId));
  }

  private static String workflowId(long deviceId) {
    return "device-" + deviceId;
  }
}
