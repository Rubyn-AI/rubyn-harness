// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { trapModalFocus } from "./accessibility";

describe("modal keyboard containment", () => {
  beforeEach(() => {
    document.body.innerHTML = '<button id="outside">Outside</button><section role="dialog" aria-modal="true"><button id="first">First</button><input id="middle"><button id="last">Last</button></section>';
  });

  it("wraps forward focus from the final control", () => {
    document.querySelector<HTMLElement>("#last")!.focus();
    const event = new KeyboardEvent("keydown", { key: "Tab", cancelable: true });
    trapModalFocus(event);
    expect(event.defaultPrevented).toBe(true);
    expect(document.activeElement?.id).toBe("first");
  });

  it("wraps backward focus and recovers focus from behind the modal", () => {
    document.querySelector<HTMLElement>("#first")!.focus();
    trapModalFocus(new KeyboardEvent("keydown", { key: "Tab", shiftKey: true, cancelable: true }));
    expect(document.activeElement?.id).toBe("last");

    document.querySelector<HTMLElement>("#outside")!.focus();
    trapModalFocus(new KeyboardEvent("keydown", { key: "Tab", cancelable: true }));
    expect(document.activeElement?.id).toBe("first");
  });
});
