package arena.junit;

import arena.junit.playbook.PlaybookInvocationExtension;
import java.lang.annotation.ElementType;
import java.lang.annotation.Repeatable;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;
import org.junit.jupiter.api.extension.ExtendWith;

@Target({ElementType.METHOD, ElementType.TYPE})
@Retention(RetentionPolicy.RUNTIME)
@Repeatable(Playbooks.class)
@ExtendWith(PlaybookInvocationExtension.class)
public @interface Playbook {
  Class<? extends arena.junit.playbook.Playbook> value();
}
