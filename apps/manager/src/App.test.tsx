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
    vscodeEditors: vi.fn(),
    vscodeInstall: vi.fn(),
    vscodeUninstall: vi.fn(),
    terminal: vi.fn(),
    installation: vi.fn(),
    desktopProfiles: vi.fn(),
    desktopOpen: vi.fn(),
    desktopRemove: vi.fn(),
    desktopCloneConfig: vi.fn(),
    desktopCloneRuntime: vi.fn(),
    checkUpdate: vi.fn(),
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
  vi.mocked(api.vscodeEditors).mockResolvedValue([]);
  vi.mocked(api.desktopProfiles).mockResolvedValue(sample.desktop_profiles);
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

describe("desktop", () => {
  const openDesktop = async () => {
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(screen.getByRole("button", { name: /Desktop/ }));
    await screen.findByText("design");
  };
  const row = (name: string) =>
    screen.getByText(name, { selector: "strong" }).closest(".session-row") as HTMLElement;

  it("is page four: ⌘4 opens it and ⌘1 goes back", async () => {
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.keyDown(window, { key: "4", metaKey: true });
    expect(await screen.findByText("design")).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Claude Desktop profiles");
    fireEvent.keyDown(window, { key: "1", metaKey: true });
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Your accounts");
  });

  it("lists profiles with what local records say about them", async () => {
    await openDesktop();
    expect(within(row("design")).getByText("designer@example.test")).toBeTruthy();
    expect(within(row("fresh")).getByText("Not signed in")).toBeTruthy();
    expect(screen.getByText(/Claude Desktop's own profile is on this Mac/)).toBeTruthy();
  });

  it("adds a profile, seeded only when asked, and opens it", async () => {
    vi.mocked(api.desktopOpen).mockResolvedValue(sample.desktop_profiles);
    await openDesktop();
    for (const [name, seeded, action] of [
      ["plain", false, "add"],
      ["seeded", true, "add_seeded"],
    ] as const) {
      fireEvent.change(screen.getByLabelText("New profile name"), { target: { value: name } });
      const seed = screen.getByRole("checkbox", { name: /MCP servers and preferences/ });
      if (seeded !== (seed as HTMLInputElement).checked) fireEvent.click(seed);
      fireEvent.click(screen.getAllByRole("button", { name: "Add profile" })[0]);
      const dialog = await screen.findByRole("dialog");
      fireEvent.click(within(dialog).getByRole("button", { name: "Add profile" }));
      await waitFor(() => expect(api.desktopOpen).toHaveBeenLastCalledWith(name, action, false));
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    }
    expect(screen.getByText(/^Added seeded and opened Claude Desktop/)).toBeTruthy();
  });

  it("refuses a reserved name before anything runs", async () => {
    await openDesktop();
    fireEvent.change(screen.getByLabelText("New profile name"), { target: { value: "default" } });
    fireEvent.click(screen.getByRole("button", { name: "Add profile" }));
    expect((await screen.findByRole("alert")).textContent).toContain("reserved");
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("opens anyway only after being told another Claude Desktop is open", async () => {
    vi.mocked(api.desktopOpen)
      .mockRejectedValueOnce("desktop_running")
      .mockResolvedValue(sample.desktop_profiles);
    await openDesktop();
    fireEvent.click(within(row("fresh")).getByRole("button", { name: "Open" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Open" }));
    await waitFor(() => expect(api.desktopOpen).toHaveBeenCalledWith("fresh", "open", false));
    expect((await within(dialog).findByRole("alert")).textContent).toContain("Another Claude Desktop");
    fireEvent.click(within(dialog).getByRole("button", { name: "Open anyway" }));
    await waitFor(() => expect(api.desktopOpen).toHaveBeenLastCalledWith("fresh", "open", true));
  });

  it("reads the list again when adding fails after the profile was made", async () => {
    vi.mocked(api.desktopProfiles)
      .mockResolvedValueOnce(sample.desktop_profiles)
      .mockResolvedValue({
        ...sample.desktop_profiles,
        profiles: [...sample.desktop_profiles.profiles, { name: "late", signed_in: false, email: null }],
      });
    vi.mocked(api.desktopOpen).mockRejectedValue("desktop_launch_failed");
    await openDesktop();
    fireEvent.change(screen.getByLabelText("New profile name"), { target: { value: "late" } });
    fireEvent.click(screen.getByRole("button", { name: "Add profile" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Add profile" }));
    expect((await within(dialog).findByRole("alert")).textContent).toContain("could not be opened");
    expect(await screen.findByText("late", { selector: "strong" })).toBeTruthy();
  });

  it("offers seeding only when the app's own profile is there", async () => {
    vi.mocked(api.desktopProfiles).mockResolvedValue({
      ...sample.desktop_profiles,
      app_profile_exists: false,
    });
    await openDesktop();
    expect(screen.queryByRole("checkbox", { name: /MCP servers/ })).toBeNull();
  });

  it("copies the app's MCP config into a profile", async () => {
    vi.mocked(api.desktopCloneConfig).mockResolvedValue(undefined);
    await openDesktop();
    fireEvent.click(within(row("fresh")).getByRole("button", { name: "Copy into…" }));
    const dialog = await screen.findByRole("dialog");
    const from = within(dialog).getByRole("combobox", { name: "From" }) as HTMLSelectElement;
    // The app's own profile first, then the others, never the profile itself.
    expect([...from.options].map((o) => o.textContent)).toStrictEqual([
      "Claude Desktop's own profile",
      "design",
    ]);
    fireEvent.click(within(dialog).getByRole("button", { name: "Copy" }));
    await waitFor(() => expect(api.desktopCloneConfig).toHaveBeenCalledWith("fresh", null, false));
    expect(
      await screen.findByText(
        "Copied the MCP servers and preferences of Claude Desktop's own profile into fresh.",
      ),
    ).toBeTruthy();
  });

  it("asks again before replacing a runtime, and says what the disk lost", async () => {
    vi.mocked(api.desktopCloneRuntime)
      .mockRejectedValueOnce("desktop_already_has")
      .mockResolvedValue(sample.runtime_cloned);
    await openDesktop();
    fireEvent.click(within(row("fresh")).getByRole("button", { name: "Copy into…" }));
    // A new choice is a new dialog, so it is looked up each time.
    const dialog = () => screen.getByRole("dialog");
    await screen.findByRole("dialog");
    fireEvent.change(within(dialog()).getByRole("combobox", { name: "What" }), {
      target: { value: "runtime" },
    });
    fireEvent.change(within(dialog()).getByRole("combobox", { name: "From" }), {
      target: { value: "design" },
    });
    fireEvent.click(within(dialog()).getByRole("button", { name: "Copy" }));
    await waitFor(() =>
      expect(api.desktopCloneRuntime).toHaveBeenCalledWith("fresh", "design", false),
    );
    fireEvent.click(await within(dialog()).findByRole("button", { name: "Replace" }));
    await waitFor(() =>
      expect(api.desktopCloneRuntime).toHaveBeenLastCalledWith("fresh", "design", true),
    );
    expect(
      await screen.findByText(/^Cloned 10\.0 GB of runtime from design into fresh\. The disk lost 0\.0 GB\./),
    ).toBeTruthy();
  });

  it("drops a refusal once the choice it was about changes", async () => {
    vi.mocked(api.desktopCloneConfig).mockRejectedValue("desktop_already_has");
    await openDesktop();
    fireEvent.click(within(row("fresh")).getByRole("button", { name: "Copy into…" }));
    fireEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Copy" }));
    expect(await within(screen.getByRole("dialog")).findByRole("alert")).toBeTruthy();
    fireEvent.change(within(screen.getByRole("dialog")).getByRole("combobox", { name: "From" }), {
      target: { value: "design" },
    });
    expect(within(screen.getByRole("dialog")).queryByRole("alert")).toBeNull();
    expect(within(screen.getByRole("dialog")).getByRole("button", { name: "Copy" })).toBeTruthy();
  });

  it("moves a profile to the Trash after confirming", async () => {
    vi.mocked(api.desktopRemove).mockResolvedValue({
      ...sample.desktop_profiles,
      profiles: [sample.desktop_profiles.profiles[1]],
    });
    await openDesktop();
    fireEvent.click(within(row("design")).getByRole("button", { name: "Move to Trash…" }));
    expect(api.desktopRemove).not.toHaveBeenCalled();
    fireEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Move to Trash" }));
    await waitFor(() => expect(api.desktopRemove).toHaveBeenCalledWith("design"));
    await waitFor(() => expect(screen.queryByText("design", { selector: "strong" })).toBeNull());
  });
});

describe("updates", () => {
  const check = async () => {
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(screen.getByRole("button", { name: "Check for updates…" }));
    expect(api.checkUpdate).not.toHaveBeenCalled();
    fireEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Check" }));
    await waitFor(() => expect(api.checkUpdate).toHaveBeenCalled());
  };

  it("names each CLI that is behind, and how it is updated", async () => {
    vi.mocked(api.checkUpdate).mockResolvedValue({
      ...sample.update_check,
      bundled: "0.21.0",
      bundled_outdated: true,
    });
    await check();
    const notice = (await screen.findByText(/^claude-acc 0\.22\.0 is out/)).textContent!;
    expect(notice).toContain("this app's CLI is 0.21.0");
    expect(notice).toContain("installed CLI is 0.21.0. Run claude-acc update");
  });

  it("says nothing is behind only when every known version is current", async () => {
    vi.mocked(api.checkUpdate).mockResolvedValue({
      ...sample.update_check,
      installed: null,
      installed_outdated: false,
    });
    await check();
    expect(
      await screen.findByText(
        "No CLI here is known to be behind: the latest claude-acc release is 0.22.0.",
      ),
    ).toBeTruthy();
  });

  it("does not call an unknown version up to date", async () => {
    vi.mocked(api.checkUpdate).mockResolvedValue({
      ...sample.update_check,
      bundled: null,
      installed: null,
      installed_outdated: false,
    });
    await check();
    expect(await screen.findByText(/did not report its version/)).toBeTruthy();
    expect(screen.queryByText(/known to be behind/)).toBeNull();
  });
});

describe("vs code", () => {
  const openProjects = async () => {
    render(<App />);
    await screen.findByText("developer@example.test");
    fireEvent.click(screen.getByRole("button", { name: /Projects/ }));
  };

  it("sets the editors up, replacing another tool's launcher only when ticked", async () => {
    vi.mocked(api.vscodeEditors).mockResolvedValue(
      sample.editors.map((e) => ({ ...e, state: e.name === "Cursor" ? "foreign" : "unset" })),
    );
    vi.mocked(api.vscodeInstall).mockResolvedValue(sample.editors);
    await openProjects();
    expect(await screen.findByText(/off: the extension's own UI ignores/)).toBeTruthy();
    expect(screen.getByText(/profiles other than the default/)).toBeTruthy();
    // Nothing is ours yet, so there is nothing to remove.
    expect((screen.getByRole("button", { name: "Remove…" }) as HTMLButtonElement).disabled).toBe(true);
    for (const force of [false, true]) {
      fireEvent.click(screen.getByRole("button", { name: "Set up…" }));
      const dialog = await screen.findByRole("dialog");
      const box = within(dialog).getByRole("checkbox", { name: /another tool set/ }) as HTMLInputElement;
      expect(box.checked).toBe(false);
      if (force) fireEvent.click(box);
      fireEvent.click(within(dialog).getByRole("button", { name: "Set up" }));
      await waitFor(() => expect(api.vscodeInstall).toHaveBeenLastCalledWith(force));
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    }
    expect((screen.getByRole("button", { name: "Remove…" }) as HTMLButtonElement).disabled).toBe(false);
    expect(screen.getByText("Restart the editor for it to take effect.")).toBeTruthy();
    // Replacing is a choice made each time, not one that sticks.
    fireEvent.click(screen.getByRole("button", { name: "Set up…" }));
    const again = await screen.findByRole("dialog");
    expect(
      (within(again).getByRole("checkbox", { name: /another tool set/ }) as HTMLInputElement).checked,
    ).toBe(false);
  });

  it("removes the integration and offers no replace box without another launcher", async () => {
    vi.mocked(api.vscodeEditors).mockResolvedValue([{ ...sample.editors[0] }]);
    vi.mocked(api.vscodeUninstall).mockResolvedValue([{ ...sample.editors[0], state: "unset" }]);
    await openProjects();
    fireEvent.click(await screen.findByRole("button", { name: "Set up…" }));
    expect(within(await screen.findByRole("dialog")).queryByRole("checkbox")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    fireEvent.click(screen.getByRole("button", { name: "Remove…" }));
    fireEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Remove" }));
    await waitFor(() => expect(api.vscodeUninstall).toHaveBeenCalled());
    expect(await screen.findByText(/off: the extension's own UI ignores/)).toBeTruthy();
  });

  it("reads the editors again after a failure, and hints a restart only after a change", async () => {
    const unset = { ...sample.editors[0], state: "unset" as const };
    vi.mocked(api.vscodeEditors)
      .mockResolvedValueOnce([unset])
      .mockResolvedValue([sample.editors[0]]);
    vi.mocked(api.vscodeInstall)
      .mockRejectedValueOnce("storage_write_failed")
      .mockResolvedValue([sample.editors[0]]);
    await openProjects();
    fireEvent.click(await screen.findByRole("button", { name: "Set up…" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Set up" }));
    await within(dialog).findByRole("alert");
    // Partly written: the list now shows what is on disk.
    await waitFor(() => expect(api.vscodeEditors).toHaveBeenCalledTimes(2));
    expect(await screen.findByText(/on: the extension runs the agent/)).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "Set up" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    // Nothing changed this time, so there is nothing to restart for.
    expect(screen.queryByText("Restart the editor for it to take effect.")).toBeNull();
  });

  it("says when no editor is installed", async () => {
    await openProjects();
    expect(await screen.findByText(/No VS Code, VS Code Insiders, VSCodium or Cursor/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Set up…" })).toBeNull();
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
