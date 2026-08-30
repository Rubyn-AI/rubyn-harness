export function createExclusiveTask(task: () => Promise<void>) {
  let running = false;

  return async (): Promise<boolean> => {
    if (running) return false;
    running = true;
    try {
      await task();
      return true;
    } finally {
      running = false;
    }
  };
}
