using System.Collections.Generic;
using ArenaDotnet.Xunit.Playbook;

namespace ArenaExamples.Test.Shared;

public static class Playbooks
{
    public sealed class CalibrationHappyPathPlaybook : ManagedHttpPlaybook
    {
        public CalibrationHappyPathPlaybook(string dependencyIdentifier)
            : base("test-calibration-api-happy-path", dependencyIdentifier,
                BuildMappings(dep => dep
                    .Post("/api/v1/validate")
                    .WillReturn(HttpResponse.OkJson(new { valid = true }))))
        {
        }
    }

    public sealed class CalibrationOutagePlaybook : ManagedHttpPlaybook
    {
        public CalibrationOutagePlaybook(string dependencyIdentifier)
            : base("test-calibration-api-error-path", dependencyIdentifier,
                BuildMappings(dep => dep
                    .Post("/api/v1/validate")
                    .WillReturn(HttpResponse.ServerError())))
        {
        }
    }

    public sealed class CalibrationFlakyPlaybook : ManagedHttpPlaybook
    {
        public CalibrationFlakyPlaybook(string dependencyIdentifier)
            : base("test-calibration-api-flaky-path", dependencyIdentifier,
                BuildMappings(dep => dep
                    .Post("/api/v1/validate")
                    .WillReturn(HttpResponse.ServerError())
                    .ThenReturn(HttpResponse.Status(503))
                    .ThenReturn(HttpResponse.OkJson(new { valid = true }))))
        {
        }
    }

    public sealed class ResetValidationDbPlaybook : ManagedMssqlPlaybook
    {
        public ResetValidationDbPlaybook(string dependencyIdentifier)
            : base("test-validation-db-scoped", dependencyIdentifier)
        {
        }
    }

    public sealed class ResetReadingsDbPlaybook : ManagedPostgresPlaybook
    {
        public ResetReadingsDbPlaybook(string dependencyIdentifier)
            : base("test-readings-db-scoped", dependencyIdentifier)
        {
        }
    }

    public sealed class EventsPurgePlaybook : ManagedLocalstackPlaybook
    {
        public EventsPurgePlaybook(string dependencyIdentifier)
            : base("test-events-purge", dependencyIdentifier)
        {
        }
    }

    public sealed class TrafficVerifyAtLeast : ManagedHttpPlaybook
    {
        public TrafficVerifyAtLeast(string dependencyIdentifier)
            : base("test-traffic-verify", dependencyIdentifier, new List<object>())
        {
        }
    }

    private static List<object> BuildMappings(System.Func<HttpPlaybookBuilder, HttpMappingBuilder> configure)
    {
        var builder = new HttpPlaybookBuilder(string.Empty);
        var mapping = configure(builder);
        return mapping.BuildMappings();
    }
}
