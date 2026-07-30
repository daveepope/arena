using ArenaXunit.Playbook;

namespace ArenaExamples.Test.Shared;

public class CalibrationApiHappyPathPlaybook : ManagedHttpPlaybook
{
    public CalibrationApiHappyPathPlaybook(string dependencyIdentifier, string validatePath)
        : base("example-api-calibration-api-happy-path", dependencyIdentifier,
            new HttpPlaybookBuilder(dependencyIdentifier)
                .Post(validatePath)
                .WillReturn(HttpResponse.OkJson(new { valid = true })))
    {
    }
}

public class CalibrationApiErrorPathPlaybook : ManagedHttpPlaybook
{
    public CalibrationApiErrorPathPlaybook(string dependencyIdentifier, string validatePath)
        : base("example-api-calibration-api-error-path", dependencyIdentifier,
            new HttpPlaybookBuilder(dependencyIdentifier)
                .Post(validatePath)
                .WillReturn(HttpResponse.ServerError()))
    {
    }
}

public class CalibrationApiFlakyPlaybook : ManagedHttpPlaybook
{
    public CalibrationApiFlakyPlaybook(string dependencyIdentifier, string validatePath)
        : base("example-api-calibration-api-flaky-path", dependencyIdentifier,
            new HttpPlaybookBuilder(dependencyIdentifier)
                .Post(validatePath)
                .WillReturn(HttpResponse.ServerError())
                .WillReturn(new HttpResponseObj(503, null, "text/plain"))
                .WillReturn(HttpResponse.OkJson(new { valid = true })))
    {
    }
}
