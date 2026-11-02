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
    removeAccount: vi.fn(),
    importAccount: vi.fn(),
    pinAccount: vi.fn(),
    cloneSettings: vi.fn(),
    installStatusline: vi.fn(),
    bindings: vi.fn(),
    bind: vi.fn(),
    sessions: vi.fn(),
    copySession: vi.fn(),
    resumeHook: vi.fn(),
    setResumeHook: vi.fn(),
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
  vi.mocked(api.resumeHook).mockResolvedValue(true);
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

  it("moves a managed account to the Trash only after confirmation", async () => {
    const removed = snapshot();
    removed.revision = 4;
    removed.accounts = removed.accounts.filter((a) => a.name !== "personal");
    vi.mocked(api.removeAccount).mockResolvedValue(removed);
    render(<App />);
    await screen.findByText("developer@example.test");
    // Every managed account offers it; the standard `~/.claude` one does not.
    expect(screen.getAllByText("Move to Trash…")).toHaveLength(2);
    expect(document.querySelectorAll("article.account-card")).toHaveLength(3);
    fireEvent.click(within(card("personal")).getByText("Move to Trash…"));
    expect(api.removeAccount).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Move to Trash" }));
    await waitFor(() => expect(api.removeAccount).toHaveBeenCalledWith("personal"));
    await waitFor(() => expect(screen.queryByText("personal", { selector: "h3" })).toBeNull());
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("keeps the account and says why when the Trash refuses it", async () => {
    vi.mocked(api.removeAccount).mockRejectedValue("trash_unavailable");
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(within(card("personal")).getByText("Move to Trash…"));
    fireEvent.click(screen.getByRole("button", { name: "Move to Trash" }));
    const dialog = await screen.findByRole("dialog");
    expect((await within(dialog).findByRole("alert")).textContent).toContain(
      "nothing was removed",
    );
    expect(card("personal")).toBeTruthy();
  });

  it("re-pins a drifted account only with force, and unblocks its launch", async () => {
    const pinned = snapshot();
    pinned.revision = 4;
    pinned.accounts.find((a) => a.name === "personal")!.lock = "matched";
    vi.mocked(api.pinAccount).mockResolvedValue(pinned);
    render(<App />);
    await screen.findByText("developer@example.test");
    // A matched pin offers neither action.
    expect(within(card("work")).queryByText(/pin/i)).toBeNull();
    const launch = () =>
      within(card("personal")).getByRole("button", { name: /Open Claude/ }) as HTMLButtonElement;
    expect(launch().disabled).toBe(true);
    fireEvent.click(within(card("personal")).getByText("Re-pin to signed-in identity…"));
    expect(api.pinAccount).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Re-pin" }));
    await waitFor(() => expect(api.pinAccount).toHaveBeenCalledWith("personal", true));
    await waitFor(() => expect(launch().disabled).toBe(false));
  });

  it("pins an unpinned account without force", async () => {
    const unpinned = snapshot();
    unpinned.accounts.find((a) => a.name === "work")!.lock = "unpinned";
    vi.mocked(api.snapshot).mockResolvedValue(unpinned);
    vi.mocked(api.pinAccount).mockResolvedValue(snapshot());
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(within(card("work")).getByText("Pin identity…"));
    fireEvent.click(screen.getByRole("button", { name: "Pin" }));
    await waitFor(() => expect(api.pinAccount).toHaveBeenCalledWith("work", false));
  });

  it("says why an account that never signed in cannot be pinned", async () => {
    const unpinned = snapshot();
    unpinned.accounts.find((a) => a.name === "work")!.lock = "unpinned";
    vi.mocked(api.snapshot).mockResolvedValue(unpinned);
    vi.mocked(api.pinAccount).mockRejectedValue("not_signed_in");
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(within(card("work")).getByText("Pin identity…"));
    fireEvent.click(screen.getByRole("button", { name: "Pin" }));
    const dialog = await screen.findByRole("dialog");
    expect((await within(dialog).findByRole("alert")).textContent).toContain(
      "never signed in",
    );
  });

  it("copies the standard settings into a managed account and lists what came over", async () => {
    vi.mocked(api.cloneSettings).mockResolvedValue(["settings.json", "agents/"]);
    render(<App />);
    await screen.findByText("developer@example.test");
    // The standard account is the source, so it is not offered.
    expect(within(card("Standard account")).queryByText(/Copy settings/)).toBeNull();
    fireEvent.click(within(card("work")).getByText("Copy settings from the standard account…"));
    expect(api.cloneSettings).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Copy settings" }));
    await waitFor(() => expect(api.cloneSettings).toHaveBeenCalledWith("work"));
    expect(await screen.findByText("Copied settings.json and agents/ into work.")).toBeTruthy();
  });

  it("says so when cloning settings had nothing to copy", async () => {
    vi.mocked(api.cloneSettings).mockResolvedValue([]);
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(within(card("work")).getByText("Copy settings from the standard account…"));
    fireEvent.click(screen.getByRole("button", { name: "Copy settings" }));
    expect(await screen.findByText(/^Nothing to copy: work already has these/)).toBeTruthy();
  });

  it("installs the status line, asking again before replacing one of the user's own", async () => {
    vi.mocked(api.installStatusline)
      .mockRejectedValueOnce("statusline_exists")
      .mockResolvedValue(undefined);
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(within(card("work")).getByText("Install status line…"));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Install" }));
    await waitFor(() => expect(api.installStatusline).toHaveBeenCalledWith("work", false));
    expect((await within(dialog).findByRole("alert")).textContent).toContain(
      "status line of its own",
    );
    fireEvent.click(within(dialog).getByRole("button", { name: "Replace status line" }));
    await waitFor(() => expect(api.installStatusline).toHaveBeenLastCalledWith("work", true));
    expect(await screen.findByText(/^Status line installed for work\./)).toBeTruthy();
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

  it("adds an account plain, or seeded from the standard one when asked", async () => {
    render(<App />);
    await screen.findByText("developer@example.test");
    for (const [seeded, action] of [
      [false, "add"],
      [true, "add_seeded"],
    ] as const) {
      fireEvent.click(screen.getByRole("button", { name: "Add account" }));
      fireEvent.change(screen.getByPlaceholderText("e.g. work"), {
        target: { value: "fresh" },
      });
      fireEvent.click(screen.getByRole("button", { name: "Continue" }));
      const dialog = await screen.findByRole("dialog");
      const box = within(dialog).getByRole("checkbox", {
        name: /standard account's settings/,
      }) as HTMLInputElement;
      // Seeding is opt-in, as `--seed` is.
      expect(box.checked).toBe(false);
      if (seeded) fireEvent.click(box);
      fireEvent.click(within(dialog).getByRole("button", { name: "Create & authenticate" }));
      await waitFor(() => expect(api.terminal).toHaveBeenLastCalledWith("fresh", null, action));
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    }
  });

  const startImport = async (name: string) => {
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(screen.getByRole("button", { name: "Add account" }));
    fireEvent.click(screen.getByRole("button", { name: "Import an existing folder instead" }));
    fireEvent.change(screen.getByPlaceholderText("e.g. work"), { target: { value: name } });
  };

  it("imports a copy of a chosen folder unless a move is asked for", async () => {
    const imported = snapshot();
    imported.revision = 9;
    imported.accounts.push({ ...imported.accounts[1], name: "legacy", is_default: false });
    vi.mocked(api.importAccount).mockResolvedValue(imported);
    vi.mocked(api.chooseDirectory).mockResolvedValue("/Users/me/.claude-work");
    await startImport("legacy");
    fireEvent.click(screen.getByRole("button", { name: "Browse…" }));
    await waitFor(() =>
      expect((screen.getByPlaceholderText("~/.claude-work") as HTMLInputElement).value).toBe(
        "/Users/me/.claude-work",
      ),
    );
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    const dialog = await screen.findByRole("dialog");
    const move = within(dialog).getByRole("checkbox", { name: /Move the folder/ }) as HTMLInputElement;
    expect(move.checked).toBe(false);
    fireEvent.click(within(dialog).getByRole("button", { name: "Import a copy" }));
    await waitFor(() =>
      expect(api.importAccount).toHaveBeenCalledWith("legacy", "/Users/me/.claude-work", false),
    );
    expect(await screen.findByText("legacy", { selector: "h3" })).toBeTruthy();
    expect(screen.getByText(/^Imported legacy\./)).toBeTruthy();
    expect(api.terminal).not.toHaveBeenCalled();
  });

  it("moves the folder only when the box is ticked, and shows a refusal", async () => {
    vi.mocked(api.importAccount).mockRejectedValue("import_source_refused");
    await startImport("legacy");
    fireEvent.change(screen.getByPlaceholderText("~/.claude-work"), {
      target: { value: "~/.claude-work" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("checkbox", { name: /Move the folder/ }));
    fireEvent.click(within(dialog).getByRole("button", { name: "Move and import" }));
    await waitFor(() =>
      expect(api.importAccount).toHaveBeenCalledWith("legacy", "~/.claude-work", true),
    );
    expect((await within(dialog).findByRole("alert")).textContent).toContain(
      "cannot be imported",
    );
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

  const openSessions = async () => {
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(screen.getByRole("button", { name: /Sessions/ }));
    await screen.findAllByText(sample.sessions[0].id);
  };
  const copyButton = () =>
    screen.getByRole("button", {
      name: `Copy session ${sample.sessions[0].id} from work to another account`,
    }) as HTMLButtonElement;

  it("copies a session to another available account and shows the new list", async () => {
    const original = sample.sessions[0];
    vi.mocked(api.sessions).mockResolvedValue([original]);
    vi.mocked(api.copySession).mockResolvedValue([
      { ...original, duplicate: true },
      { ...original, account: "personal", duplicate: true },
    ]);
    await openSessions();
    fireEvent.click(copyButton());
    const dialog = await screen.findByRole("dialog");
    // Not its own account, and not the unavailable standard one.
    const options = within(dialog)
      .getAllByRole("option")
      .map((o) => o.textContent);
    expect(options).toStrictEqual(["personal"]);
    expect(within(dialog).queryByText(/already has a copy/)).toBeNull();
    fireEvent.click(within(dialog).getByRole("button", { name: "Copy session" }));
    await waitFor(() =>
      expect(api.copySession).toHaveBeenCalledWith(
        "work",
        original.project,
        original.id,
        "personal",
        false,
      ),
    );
    await waitFor(() => expect(document.querySelectorAll(".session-row")).toHaveLength(2));
    expect(screen.getByText(/claude-acc run personal --resume/).textContent).toContain(
      original.id,
    );
  });

  it("asks before replacing a copy the destination already holds", async () => {
    const original = sample.sessions[0];
    const theirs = { ...original, account: "personal", duplicate: true };
    vi.mocked(api.sessions).mockResolvedValue([{ ...original, duplicate: true }, theirs]);
    vi.mocked(api.copySession).mockResolvedValue([original, theirs]);
    await openSessions();
    fireEvent.click(copyButton());
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/already has a copy/)).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "Replace copy" }));
    await waitFor(() =>
      expect(api.copySession).toHaveBeenCalledWith(
        "work",
        original.project,
        original.id,
        "personal",
        true,
      ),
    );
  });

  it("reloads the list when the destination gained a copy, so replacing is one click", async () => {
    const original = sample.sessions[0];
    const theirs = { ...original, account: "personal", duplicate: true };
    vi.mocked(api.sessions)
      .mockResolvedValueOnce([original])
      .mockResolvedValue([{ ...original, duplicate: true }, theirs]);
    vi.mocked(api.copySession).mockRejectedValue("session_exists");
    await openSessions();
    fireEvent.click(copyButton());
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Copy session" }));
    expect((await within(dialog).findByRole("alert")).textContent).toContain(
      "already has a copy",
    );
    expect(api.sessions).toHaveBeenCalledTimes(2);
    expect(await within(dialog).findByRole("button", { name: "Replace copy" })).toBeTruthy();
  });

  it("offers no copy when no other account is available", async () => {
    const alone = snapshot();
    alone.accounts.find((a) => a.name === "personal")!.available = false;
    vi.mocked(api.snapshot).mockResolvedValue(alone);
    vi.mocked(api.sessions).mockResolvedValue([sample.sessions[0]]);
    await openSessions();
    expect(copyButton().disabled).toBe(true);
  });

  it("turns the wrapper's resume check off after confirming, and shows what was stored", async () => {
    vi.mocked(api.setResumeHook).mockResolvedValue(false);
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(screen.getByRole("button", { name: /Sessions/ }));
    expect(await screen.findByText(/^On: claude --resume/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Turn off…" }));
    expect(api.setResumeHook).not.toHaveBeenCalled();
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Turn off" }));
    await waitFor(() => expect(api.setResumeHook).toHaveBeenCalledWith(false));
    expect(await screen.findByText(/^Off: claude --resume/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Turn on…" })).toBeTruthy();
  });

  it("shows a stored off and turns it back on", async () => {
    vi.mocked(api.resumeHook).mockResolvedValue(false);
    vi.mocked(api.setResumeHook).mockResolvedValue(true);
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(screen.getByRole("button", { name: /Sessions/ }));
    fireEvent.click(await screen.findByRole("button", { name: "Turn on…" }));
    fireEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Turn on" }));
    await waitFor(() => expect(api.setResumeHook).toHaveBeenCalledWith(true));
    expect(await screen.findByText(/^On: claude --resume/)).toBeTruthy();
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
