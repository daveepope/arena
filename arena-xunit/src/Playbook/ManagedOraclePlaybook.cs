using System;

namespace ArenaDotnet.Xunit.Playbook;

public abstract class ManagedOraclePlaybook : ManagedPlaybook
{
    protected ManagedOraclePlaybook(string identifier, string dependencyIdentifier)
        : base(identifier, dependencyIdentifier)
    {
    }

    internal override string Kind => "oracle";

    internal override ActivePlaybook WrapHandle(IntPtr handle) => new ActiveOraclePlaybook(handle);
}
