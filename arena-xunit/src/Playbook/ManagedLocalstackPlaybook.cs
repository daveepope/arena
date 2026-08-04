using System;

namespace ArenaDotnet.Xunit.Playbook;

public abstract class ManagedLocalstackPlaybook : ManagedPlaybook
{
    protected ManagedLocalstackPlaybook(string identifier, string dependencyIdentifier)
        : base(identifier, dependencyIdentifier)
    {
    }

    internal override string Kind => "localstack";

    internal override ActivePlaybook WrapHandle(IntPtr handle) => new ActiveLocalstackPlaybook(handle);
}
