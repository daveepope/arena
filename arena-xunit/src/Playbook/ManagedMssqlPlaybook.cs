using System;

namespace ArenaDotnet.Xunit.Playbook;

public abstract class ManagedMssqlPlaybook : ManagedPlaybook
{
    protected ManagedMssqlPlaybook(string identifier, string dependencyIdentifier)
        : base(identifier, dependencyIdentifier)
    {
    }

    internal override string Kind => "mssql";

    internal override ActivePlaybook WrapHandle(IntPtr handle) => new ActiveMssqlPlaybook(handle);
}
