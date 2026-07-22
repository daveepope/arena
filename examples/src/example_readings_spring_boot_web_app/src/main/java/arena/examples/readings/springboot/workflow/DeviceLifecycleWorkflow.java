package arena.examples.readings.springboot.workflow;

import io.temporal.workflow.QueryMethod;
import io.temporal.workflow.SignalMethod;
import io.temporal.workflow.WorkflowInterface;
import io.temporal.workflow.WorkflowMethod;

@WorkflowInterface
public interface DeviceLifecycleWorkflow {

  @WorkflowMethod
  void run(long deviceId);

  @SignalMethod
  void requestTransition(DeviceState target);

  @SignalMethod
  void stop();

  @QueryMethod
  DeviceSnapshot snapshot();
}
