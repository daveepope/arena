package arena.examples.readings.springboot.workflow;

import io.temporal.client.WorkflowClient;
import io.temporal.serviceclient.WorkflowServiceStubs;
import io.temporal.serviceclient.WorkflowServiceStubsOptions;
import io.temporal.worker.Worker;
import io.temporal.worker.WorkerFactory;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.SmartLifecycle;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

@Configuration
public class TemporalConfiguration implements SmartLifecycle {

  private static final String TASK_QUEUE = "device-lifecycle-task-queue";

  private WorkflowServiceStubs serviceStubs;
  private WorkerFactory workerFactory;
  private volatile boolean running = false;

  public static String taskQueue() {
    return TASK_QUEUE;
  }

  @Bean
  WorkflowServiceStubs temporalServiceStubs(@Value("${TEMPORAL_TARGET}") String target) {
    serviceStubs =
        WorkflowServiceStubs.newServiceStubs(
            WorkflowServiceStubsOptions.newBuilder().setTarget(target).build());
    return serviceStubs;
  }

  @Bean
  WorkflowClient temporalWorkflowClient(WorkflowServiceStubs stubs) {
    return WorkflowClient.newInstance(stubs);
  }

  @Bean
  WorkerFactory temporalWorkerFactory(WorkflowClient client) {
    workerFactory = WorkerFactory.newInstance(client);
    Worker worker = workerFactory.newWorker(TASK_QUEUE);
    worker.registerWorkflowImplementationTypes(DeviceLifecycleWorkflowImpl.class);
    worker.registerActivitiesImplementations(new DeviceActivitiesImpl());
    return workerFactory;
  }

  @Override
  public void start() {
    workerFactory.start();
    running = true;
  }

  @Override
  public void stop() {
    running = false;
    if (workerFactory != null) {
      workerFactory.shutdown();
    }
    if (serviceStubs != null) {
      serviceStubs.shutdown();
    }
  }

  @Override
  public boolean isRunning() {
    return running;
  }

  @Override
  public int getPhase() {
    return Integer.MAX_VALUE;
  }
}
