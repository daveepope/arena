using System;

namespace ArenaXunit.Playbook;

public sealed class ActiveLocalstackPlaybook : ActivePlaybook
{
    public ActiveLocalstackPlaybook(IntPtr handle) : base(handle) { }
}
