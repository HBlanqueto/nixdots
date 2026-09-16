/* islands — tiny event bus.
 *
 * Global pub/sub used to fan pref and theme changes out to every open
 * window controller without them polling.
 */

class EventBus {
  constructor() {
    this._topics = new Map();
  }

  /** on(topic, handler) -> unsubscribe function. */
  on(topic, handler) {
    let list = this._topics.get(topic);
    if (!list) {
      list = new Set();
      this._topics.set(topic, list);
    }
    list.add(handler);
    return () => list.delete(handler);
  }

  emit(topic, payload) {
    const list = this._topics.get(topic);
    if (!list) return;
    for (const handler of list) {
      try {
        handler(payload);
      } catch (e) {
        console.error("uc.islands bus handler error on", topic, e);
      }
    }
  }
}

const bus = new EventBus();
export { bus as EventBus };