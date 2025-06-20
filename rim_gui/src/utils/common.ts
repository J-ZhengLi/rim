import { installConf } from "./installConf";
import { invokeCommand } from "./invokeCommand";
import { RestrictedComponent } from "./types/Component";

/**
 * Handle the restricted components before installation,
 * as some components might need another package source.
 * 
 * @param onDefault The default callback where there aren't any restricted components.
 * @param onRestricted Callback when restricted components detected in `installConf`.
 */
export function handleRestrictedComponents(onDefault: () => void, onRestricted: () => void) {
  invokeCommand('get_restricted_components', { components: installConf.getCheckedComponents() }).then((res) => {
    const restricted = res as RestrictedComponent[];
    if (restricted.length > 0) {
      installConf.setRestrictedComponents(restricted);
      onRestricted();
    } else {
      onDefault();
    }
  });
}
