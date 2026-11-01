import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { App } from "./App";
import { api } from "./api";
import type { Session, Snapshot } from "./contract";
import { sample } from "./contract.sample";

vi.mock("./api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./api")>()),
  api: {
    snapshot: vi.fn(),
    setDefault: vi.fn(),
    bindings: vi.fn(),
    bind: vi.fn(),
    sessions: vi.fn(),
    terminal: vi.fn(),
    installation: vi.fn(),
    chooseDirectory: vi.fn(),
  },
}));

// The backend's events, delivered on demand.
const handlers = new Map<string, (e: { payload: unknown }) => void>();
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, handler: (e: { payload: unknown }) => void) => {
    handlers.set(name, handler);
    return () => handlers.delete(name);
  }),
}));
const emit = (name: string, payload: unknown) =>
  act(() => handlers.get(name)?.({ payload }));

const snapshot = (): Snapshot => structuredClone(sample.snapshot);
const card = (name: string) =>
  screen.getByText(name, { selector: "h3" }).closest("article")!;

beforeEach(() => {
  // A minute after the cached reading was taken. Only Date and intervals are
  // faked, so testing-library's own timeouts still run.
  vi.useFakeTimers({ toFake: ["Date", "setInterval", "clearInterval"] });
  vi.setSystemTime(1_700_000_060 * 1000);
  localStorage.clear();
  vi.mocked(api.snapshot).mockResolvedValue(snapshot());
  vi.mocked(api.bindings).mockResolvedValue({ bindings: [], selected: null });
  vi.mocked(api.sessions).mockResolvedValue([]);
  vi.mocked(api.terminal).mockResolvedValue({ state: "started" });
  vi.mocked(api.installation).mockResolvedValue(sample.installation);
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  handlers.clear();
});

describe("accounts", () => {
  it("shows backend fields and ages usage against the clock", async () => {
    render(<App />);
    expect(screen.getByRole("status").textContent).toContain("Reading");
    await screen.findByText("developer@example.test");
    const work = within(card("work"));
    expect(work.getByText("42%")).toBeTruthy();
    // The 7-day window reset before "now": no figure, however recent the reading.
    expect(work.getByText("Reset passed")).toBeTruthy();
    expect(work.getByText("Cached by Claude Code · 1m ago")).toBeTruthy();
    expect(work.getByText("○ Local identity")).toBeTruthy();
    expect(api.snapshot).toHaveBeenCalledWith(false);

    // Regression: ages were frozen at snapshot time. Ten minutes later the
    // same reading, with no new snapshot, must read as stale.
    act(() => {
      vi.setSystemTime(1_700_000_660 * 1000);
      vi.advanceTimersByTime(30_000);
    });
    expect(work.getByText("Stale · 11m ago · refresh to update")).toBeTruthy();

    // The 5-hour window resets at 1_700_007_200; after that its 42% is gone.
    act(() => {
      vi.setSystemTime(1_700_007_200 * 1000);
      vi.advanceTimersByTime(30_000);
    });
    expect(work.queryByText("42%")).toBeNull();
    expect(work.getAllByText("Reset passed")).toHaveLength(2);
  });

  it("filters by name or email", async () => {
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.change(screen.getByLabelText("Search accounts"), {
      target: { value: "personal" },
    });
    expect(screen.queryByText("developer@example.test")).toBeNull();
    expect(screen.getByText("personal@example.test")).toBeTruthy();
  });

  it("changes the default only after confirmation, and shows the result", async () => {
    const changed = snapshot();
    changed.revision = 4;
    changed.default_account = "personal";
    vi.mocked(api.setDefault).mockResolvedValue(changed);
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(within(card("personal")).getByText("Make default…"));
    expect(api.setDefault).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Change default" }));
    await waitFor(() => expect(api.setDefault).toHaveBeenCalledWith("personal"));
    await waitFor(() =>
      expect(document.querySelector(".default-name")?.textContent).toBe("personal"),
    );
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("closes an account's action menu once used, or on a click elsewhere", async () => {
    // Regression: the <details> menu stayed open behind and after its dialog.
    render(<App />);
    await screen.findByText("developer@example.test");
    const menu = card("personal").querySelector("details")!;
    fireEvent.click(within(card("personal")).getByText("•••"));
    expect(menu.open).toBe(true);
    fireEvent.click(within(card("personal")).getByText("Details & diagnostics"));
    expect(menu.open).toBe(false);
    menu.open = true;
    fireEvent.pointerDown(document.body);
    expect(menu.open).toBe(false);
    menu.open = true;
    fireEvent.keyDown(document, { key: "Escape" });
    expect(menu.open).toBe(false);
  });

  it("keeps a failed change's dialog open with the reason", async () => {
    vi.mocked(api.setDefault).mockRejectedValue("storage_write_failed");
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(within(card("personal")).getByText("Make default…"));
    fireEvent.click(screen.getByRole("button", { name: "Change default" }));
    const dialog = await screen.findByRole("dialog");
    expect((await within(dialog).findByRole("alert")).textContent).toContain(
      "could not be saved",
    );
  });

  it("reports bridge failures and retries without inventing account data", async () => {
    vi.mocked(api.snapshot).mockRejectedValueOnce("storage_unreadable");
    render(<App />);
    expect((await screen.findByRole("alert")).textContent).toContain(
      "could not be read",
    );
    expect(screen.queryByText("developer@example.test")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await screen.findByText("developer@example.test");
  });

  it("takes snapshots from the menu bar, but never an older one", async () => {
    render(<App />);
    await screen.findByText("developer@example.test");
    const newer = snapshot();
    newer.revision = 9;
    newer.accounts[1].email = "from-tray@example.test";
    emit("snapshot", newer);
    await screen.findByText("from-tray@example.test");
    // A slower reply built earlier arrives last; it must not win.
    const older = snapshot();
    older.revision = 5;
    older.accounts[1].email = "stale@example.test";
    emit("snapshot", older);
    expect(screen.queryByText("stale@example.test")).toBeNull();
    expect(screen.getByText("from-tray@example.test")).toBeTruthy();
  });

  it("launches with an explicit account and says what is actually known", async () => {
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(within(card("work")).getByRole("button", { name: "Open Claude" }));
    await waitFor(() => expect(api.terminal).toHaveBeenCalledWith("work", null, "run"));
    expect((await screen.findByRole("status")).textContent).toContain(
      "Started in Terminal under work",
    );
  });

  it("blocks launching an account whose identity drifted from its pin", async () => {
    render(<App />);
    await screen.findByText("developer@example.test");
    const personal = within(card("personal"));
    expect(personal.getByText("Identity drift")).toBeTruthy();
    expect(
      (personal.getByRole("button", { name: "Open Claude" }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
  });

  it("refuses an invalid new account name before anything runs", async () => {
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(screen.getByRole("button", { name: "Add account" }));
    fireEvent.change(screen.getByPlaceholderText("e.g. work"), {
      target: { value: "default" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect((await screen.findByRole("alert")).textContent).toContain("reserved");
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(api.terminal).not.toHaveBeenCalled();
  });

  it("explains a fresh installation without fabricating an account", async () => {
    vi.mocked(api.snapshot).mockResolvedValue({
      ...snapshot(),
      accounts: [snapshot().accounts[0]],
    });
    render(<App />);
    await screen.findByText("No configured accounts yet");
    expect(api.terminal).not.toHaveBeenCalled();
  });
});

describe("projects", () => {
  it("uses the backend's effective account and confirms the binding", async () => {
    vi.mocked(api.chooseDirectory).mockResolvedValue("/Projects/nested");
    vi.mocked(api.bindings).mockImplementation(async (path) =>
      path ? sample.bindings : { bindings: [], selected: null },
    );
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(screen.getByRole("button", { name: /Projects/ }));
    fireEvent.click(screen.getByRole("button", { name: "Browse…" }));
    await screen.findByText("From binding: /Projects");
    fireEvent.change(screen.getByLabelText("Account for selected project"), {
      target: { value: "personal" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save binding" }));
    expect(api.bind).not.toHaveBeenCalled();
    fireEvent.click(
      within(screen.getByRole("dialog")).getByRole("button", { name: "Save binding" }),
    );
    await waitFor(() =>
      expect(api.bind).toHaveBeenCalledWith("/Projects/nested", "personal"),
    );
  });
});

describe("sessions", () => {
  it("shows an empty browser without a pretend copy button", async () => {
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(screen.getByRole("button", { name: /Sessions/ }));
    await screen.findByText("No sessions found");
    expect(screen.queryByRole("button", { name: /Copy/ })).toBeNull();
  });

  it("renders large lists a page at a time", async () => {
    const many: Session[] = Array.from({ length: 250 }, (_, i) => ({
      ...sample.sessions[0],
      id: `session-${i}`,
    }));
    vi.mocked(api.sessions).mockResolvedValue(many);
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(screen.getByRole("button", { name: /Sessions/ }));
    await screen.findByText("session-0");
    expect(document.querySelectorAll(".session-row")).toHaveLength(100);
    fireEvent.click(screen.getByRole("button", { name: /Show more \(150/ }));
    expect(document.querySelectorAll(".session-row")).toHaveLength(200);
  });

  it("retries failed discovery on the same page", async () => {
    vi.mocked(api.sessions)
      .mockRejectedValueOnce("storage_unreadable")
      .mockResolvedValue([]);
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(screen.getByRole("button", { name: /Sessions/ }));
    fireEvent.click(await screen.findByRole("button", { name: "Retry" }));
    await screen.findByText("No sessions found");
    expect(api.sessions).toHaveBeenCalledTimes(2);
  });
});
