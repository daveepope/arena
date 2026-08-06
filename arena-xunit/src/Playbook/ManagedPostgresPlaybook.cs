using System;

namespace ArenaDotnet.Xunit.Playbook;

public abstract class ManagedPostgresPlaybook : ManagedPlaybook
{
    protected ManagedPostgresPlaybook(string identifier, string dependencyIdentifier)
        : base(identifier, dependencyIdentifier)
    {
    }

    internal override string Kind => "postgres";

    internal override ActivePlaybook WrapHandle(IntPtr handle) => new ActivePostgresPlaybook(handle);
}
