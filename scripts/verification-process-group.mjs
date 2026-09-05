// Call only for a child created with detached:true, which owns its process group.
import { setTimeout as delay } from 'node:timers/promises';

export function ownedGroupCleanup(child, timeoutMs = 15000) {
  let completion;
  const group = child.pid;
  function signalGroup(signal) {
    if (!group) return false; // Spawn failed before a group was created.
    try { process.kill(-group, signal); return true; }
    catch (error) { if (error.code === 'ESRCH') return false; throw error; }
  }
  async function waitGroup(timeout) {
    const deadline = performance.now() + timeout;
    while (signalGroup(0)) {
      if (performance.now() >= deadline) return false;
      await delay(20);
    }
    return true;
  }
  return () => completion ??= (async () => {
    signalGroup('SIGTERM');
    if (!await waitGroup(timeoutMs)) {
      signalGroup('SIGKILL');
      if (!await waitGroup(5000)) throw new Error('Owned process group survived forced cleanup');
    }
  })();
}
