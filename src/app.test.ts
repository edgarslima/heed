import { afterEach, describe, expect, it, vi } from "vitest";
import { mountApp } from "./app";

describe("app shell", () => {
  afterEach(() => {
    document.body.innerHTML = "";
    vi.resetModules();
  });

  it("mountApp renders the app shell", () => {
    const root = document.createElement("div");

    mountApp(root);

    expect(root.querySelector("main.app")).not.toBeNull();
  });

  it("main entry mounts the shell into #app", async () => {
    document.body.innerHTML = '<div id="app"></div>';

    await import("./main");

    expect(document.querySelector("#app > main.app")).not.toBeNull();
  });

  it("main entry does nothing when #app is missing", async () => {
    document.body.innerHTML = "";

    await import("./main");

    expect(document.querySelector("main.app")).toBeNull();
  });
});
