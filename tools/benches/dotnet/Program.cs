using System.Diagnostics;
using System.Net.Http;
using System.Security.Cryptography;
using ArenaDotnet.Xunit;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Ffi;
using ArenaDotnet.Xunit.Playbook;
using Npgsql;

string version = args[0];
int iterations = int.Parse(args[1]);

string suffix = RandomNumberGenerator.GetHexString(8, lowercase: true);
string dbName = $"bench_{suffix}";
string dbUser = $"bench_{suffix}";
string dbPassword = RandomNumberGenerator.GetHexString(24, lowercase: true);
string connectionString = $"Host=127.0.0.1;Port=15432;Database={dbName};Username={dbUser};Password={dbPassword}";

const string BenchmarkTableSql =
    "CREATE TABLE benchmark (" +
    "id SERIAL PRIMARY KEY, " +
    "version TEXT NOT NULL, " +
    "phase TEXT NOT NULL, " +
    "duration_ms DOUBLE PRECISION NOT NULL, " +
    "recorded_at TIMESTAMPTZ NOT NULL DEFAULT now())";

var postgres = new PostgresDependencyBuilder("bench-postgres")
    .WithPort(15432)
    .WithDatabaseName(dbName)
    .WithDatabaseUsername(dbUser)
    .WithDatabasePassword(dbPassword)
    .WithStartupSqlScripts(new[] { BenchmarkTableSql })
    .Build();
var http = new HttpDependencyBuilder("bench-http").WithPort(18080).Build();

var managedPostgres = new BenchPostgresPlaybook(postgres.Identifier);
var managedHttp = new BenchHttpPlaybook(http.Identifier);

var match = new MatchBuilder("bench-match")
    .AddDependency(postgres)
    .AddDependency(http)
    .RegisterPlaybook(managedPostgres, execOnDependencyStart: true)
    .RegisterPlaybook(managedHttp, execOnDependencyStart: true)
    .Build();
var closed = new ClosedArena("bench-arena", match, ArenaLogLevel.Error);

var e2eSw = Stopwatch.StartNew();

var sw = Stopwatch.StartNew();
var arena = await closed.OpenAsync();
double openMs = sw.Elapsed.TotalMilliseconds;

double closeMs;
double[] sorted;
try
{
    using var client = new HttpClient();
    await using var conn = new NpgsqlConnection(connectionString);
    await conn.OpenAsync();

    new UnmanagedPostgresVerifyPlaybook(managedPostgres).Run(arena);
    new UnmanagedHttpVerifyPlaybook(managedHttp, client).Run(arena);

    var iterationMs = new List<double>();
    for (int n = 0; n < iterations; n++)
    {
        iterationMs.Add(await RunIterationAsync(n, arena, managedPostgres, client, conn, version));
    }
    sorted = iterationMs.OrderBy(v => v).ToArray();
}
finally
{
    sw.Restart();
    arena.Dispose();
    closeMs = sw.Elapsed.TotalMilliseconds;
}

double e2eMs = e2eSw.Elapsed.TotalMilliseconds;

double Percentile(double pct)
{
    int idx = Math.Min(sorted.Length - 1, (int)Math.Round(pct * (sorted.Length - 1)));
    return sorted[idx];
}

Console.WriteLine(
    $"version={version} open_ms={openMs:F2} iterations={iterations} " +
    $"interact_min_ms={sorted[0]:F2} interact_ms={Percentile(0.5):F2} " +
    $"interact_p95_ms={Percentile(0.95):F2} interact_max_ms={sorted[^1]:F2} " +
    $"close_ms={closeMs:F2} e2e_ms={e2eMs:F2}");

static async Task<double> RunIterationAsync(
    int n, OpenArena arena, BenchPostgresPlaybook managedPostgres, HttpClient client,
    NpgsqlConnection conn, string version)
{
    var iterSw = Stopwatch.StartNew();

    var response = await client.GetAsync("http://127.0.0.1:18080/health");
    response.EnsureSuccessStatusCode();
    double httpMs = iterSw.Elapsed.TotalMilliseconds;

    double readBackMs = await RecordAndReadBackAsync(conn, version, $"iter-{n}", httpMs);
    if (Math.Abs(readBackMs - httpMs) > 1e-6)
    {
        throw new InvalidOperationException(
            $"benchmark table read-back mismatch: wrote {httpMs} read {readBackMs}");
    }

    var activePostgres = (ActivePostgresPlaybook)managedPostgres.Run(arena);
    activePostgres.Verify("SELECT 1", 1);

    return iterSw.Elapsed.TotalMilliseconds;
}

static async Task<double> RecordAndReadBackAsync(NpgsqlConnection conn, string version, string phase, double durationMs)
{
    await using (var insert = new NpgsqlCommand(
        "INSERT INTO benchmark (version, phase, duration_ms) VALUES (@version, @phase, @duration_ms)", conn))
    {
        insert.Parameters.AddWithValue("version", version);
        insert.Parameters.AddWithValue("phase", phase);
        insert.Parameters.AddWithValue("duration_ms", durationMs);
        await insert.ExecuteNonQueryAsync();
    }

    await using var select = new NpgsqlCommand(
        "SELECT duration_ms FROM benchmark WHERE version = @version AND phase = @phase ORDER BY id DESC LIMIT 1", conn);
    select.Parameters.AddWithValue("version", version);
    select.Parameters.AddWithValue("phase", phase);
    var result = await select.ExecuteScalarAsync();
    return (double)result!;
}

sealed class BenchHttpPlaybook : ManagedHttpPlaybook
{
    public BenchHttpPlaybook(string dependencyIdentifier)
        : base(
            "bench-http-managed",
            dependencyIdentifier,
            new HttpPlaybookBuilder(dependencyIdentifier).Get("/health").WillReturn(HttpResponse.Ok()).BuildMappings())
    {
    }
}

sealed class BenchPostgresPlaybook : ManagedPostgresPlaybook
{
    public BenchPostgresPlaybook(string dependencyIdentifier)
        : base("bench-postgres-managed", dependencyIdentifier)
    {
    }
}

sealed class UnmanagedPostgresVerifyPlaybook : UnmanagedPlaybook
{
    private readonly BenchPostgresPlaybook _managed;

    public UnmanagedPostgresVerifyPlaybook(BenchPostgresPlaybook managed)
    {
        _managed = managed;
    }

    public override string Identifier => "bench-postgres-unmanaged-verify";

    public override ActivePlaybook Run(OpenArena arena)
    {
        var active = (ActivePostgresPlaybook)_managed.Run(arena);
        active.Verify("SELECT 1", 1);
        return active;
    }
}

sealed class UnmanagedHttpVerifyPlaybook : UnmanagedPlaybook
{
    private readonly BenchHttpPlaybook _managed;
    private readonly HttpClient _client;

    public UnmanagedHttpVerifyPlaybook(BenchHttpPlaybook managed, HttpClient client)
    {
        _managed = managed;
        _client = client;
    }

    public override string Identifier => "bench-http-unmanaged-verify";

    public override ActivePlaybook Run(OpenArena arena)
    {
        var response = _client.GetAsync("http://127.0.0.1:18080/health").GetAwaiter().GetResult();
        response.EnsureSuccessStatusCode();
        return _managed.Run(arena);
    }
}
