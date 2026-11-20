# Mac owner steps (run-004)

For the owner, on the MacBook Pro (Apple Silicon). No programming knowledge is
needed; you copy and paste commands into the Terminal app and send back text
files. Everything here uses **fake** game data in a folder created for the test.
Nothing touches your real World of Warcraft, Battle.net or Dropbox files, and
the app is never connected to them.

What you will produce (all start as NOT RUN until you do them):

| Row | What | Script |
|---|---|---|
| T-27-11 (macOS) | A real notification shows the name "WoW Sync" | `tools/mac/notify-check.sh` |
| T-28-13 (macOS) | One real "is there a new version" request works | `tools/mac/update-check.sh` |
| A-29 (macOS) | Background memory is at most 100 MiB, and sync starts within ten seconds | `tools/mac/mem.sh` |

Plan about 1.5 hours the first time (most of it is waiting for the build).

## 0. Before you start

- Quit WoW and Battle.net if they are open (not required, but it keeps the memory
  numbers clean). The kit never starts or stops them.
- Use a Mac that is plugged in, with no heavy apps open.
- In this document `<CHECKOUT>` means the folder that holds the WoW Sync
  project files (the one containing `README.md` and the `tools` folder). Use the
  branch or commit the orchestrator told you to use.

## 1. Install the tools (once)

1. Open **Terminal** (Spotlight: press Cmd+Space, type `Terminal`).
2. Install Apple's command line tools. Copy this and press Return:

   ```text
   xcode-select --install
   ```

   Click "Install" in the window that appears and wait. If it says they are
   already installed, that is fine.
3. Install Rust. Copy this and press Return, then press Return again to accept
   the defaults:

   ```text
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

4. **Close Terminal and open it again** so it can find Rust.
5. Optional but recommended (the safety audit step of the build needs it, and
   without it the build report says BLOCKED for that one line):

   ```text
   cargo install --locked cargo-deny
   ```

## 2. Build (once, about 20 to 40 minutes)

```text
cd <CHECKOUT>
sh tools/mac/build.sh
```

Replace `<CHECKOUT>` with the real folder, for example by typing `cd ` and
dragging the folder from Finder into the Terminal window.

The last line should be `BUILD OK`. If it stops with `ERROR:` or a failed check,
copy the last 40 lines into your report and stop. (`--skip-verify` skips the
test run and only builds; use it only if you are asked to.)

Save the whole Terminal output of this step: select all (Cmd+A), copy (Cmd+C),
and paste it into a text file called `build.txt`.

## 3. Notification check (T-27-11)

```text
cd <CHECKOUT>
sh tools/mac/notify-check.sh
```

The first run compiles a tiny test program (a few minutes). Then a macOS
notification should appear at the top right of the screen.

Look at it and tell the script **the application name** shown in the
notification (the small label next to the icon, for example "WoW Sync",
"Terminal" or "Script Editor"). Type it exactly. If nothing appeared, check
System Settings > Notifications for your Terminal app (it may be turned
off or in Focus mode), try once more, and if still nothing, type `none`.

A name other than "WoW Sync" counts as a **failure** on purpose (decision Q-63),
so please report what you see honestly. Take a screenshot of the notification
(Cmd+Shift+4) if you can.

The result is saved to `/Users/Shared/WoWSyncLab/notify-check-result.txt`.

## 4. Update check (T-28-13)

This makes exactly one anonymous request to GitHub's public release list.

```text
cd <CHECKOUT>
sh tools/mac/update-check.sh
```

The result is saved to `/Users/Shared/WoWSyncLab/update-check-result.txt` and the
last line says `verdict: PASS` or `verdict: FAIL`. If your Mac asks whether Terminal
may find devices on your network, you can answer Allow or Don't Allow; tell us
which, because Don't Allow can cause a failure.

## 5. Enrol the lab once (needed for the memory test)

```text
cd <CHECKOUT>
sh tools/mac/run.sh enroll
```

A WoW Sync window opens on fake game data. Click through the setup guide using
the default, obvious choices. In order, you will meet buttons like:
Next, Use this folder, Next, Next, "This is my ...", "These are the ...", Next,
"Create a new group", Next, "Set up this computer", "Finish and open ...". The
exact wording may differ a little; choose the option that creates a new
group on this computer and the fake install shown.

Wait until the main screen shows a **Sync now** button and a status. Look for:

- Does a small icon appear in the menu bar at the top of the screen?
- Is the window title "WoW Sync"?
- Does the window look broken, empty or cut off? Take a screenshot (Cmd+Shift+4).

Now stop the app: click the Terminal window and press **Ctrl-C**. If the window
or the process does not go away within ten seconds, note that and force-quit it
from Activity Monitor.

If setup cannot be finished (a button does nothing, the window stays blank,
the app closes by itself), copy `/Users/Shared/WoWSyncLab/main/app.log` into your report and stop here.
The memory test needs the setup done.

## 6. Memory and sync-start test (A-29)

Do not use the Mac for other things while this runs. Sleep must stay off
(System Settings > Lock Screen; or run the command with `caffeinate -i` in
front, as below).

Quick trial first (about 1 minute), to check everything works:

```text
cd <CHECKOUT>
caffeinate -i sh tools/mac/mem.sh --scenarios a --runs 1 --scale 0.1
```

Then the real measurement. Scenario `a` is fully automatic. Scenarios `b` and `c`
stop and ask you to do a few things in the app window; each prompt tells you
what to do and waits for Return. Allow about 45 minutes in total, three runs
per scenario (the default):

```text
caffeinate -i sh tools/mac/mem.sh --scenarios a,b,c
```

What the prompts ask:

- **b:** scroll the page and click "Sync now" now and then for a minute, then
  close the window with the red button. The app should keep running (a menu bar
  icon stays). If the app quits when you close the window, say so in your report.
- **c:** for one minute, open the window from the menu-bar icon and close it
  again, 20 times. Count the cycles and type the number when asked.

Each run says things like `a-run1 sync start: 3.4 s` and, at the end, prints a
summary with `verdict PASS` or `verdict FAIL`. Results are saved in a new folder
`/Users/Shared/WoWSyncLab/results/mem-<date>-<time>`.

## 7. What to send back

Send these (text is fine, no private data is in them; they contain the Mac
model and macOS version, which are wanted):

1. `build.txt` from step 2.
2. `/Users/Shared/WoWSyncLab/notify-check-result.txt` plus your answer to "what name did
   the notification show?", and a screenshot if you took one.
3. `/Users/Shared/WoWSyncLab/update-check-result.txt`.
4. The whole folder `/Users/Shared/WoWSyncLab/results/mem-...` (zip it: right-click >
   Compress).
5. Any screenshots or notes from step 5 (menu-bar icon, window look, anything that failed).
6. The sentence "I did / did not allow Terminal to access the network".

Anything that went wrong is useful; do not retry until it works without telling us
the first result.

## 8. Clean up

Run this to delete all test data (the fake lab and results) and the compiled
files:

```text
cd <CHECKOUT>
sh tools/mac/run.sh clean
rm -rf /Users/Shared/WoWSyncLab
cargo clean
```

(Send the results before running the second command. The first removes only the
fake lab; the second removes the whole test folder; the third removes the build
output inside the project folder.) If you are asked to keep Rust installed, leave
it; to remove it later run `rustup self uninstall`. The kit never changed any real
WoW, Battle.net or Dropbox file, so nothing else needs undoing.

## Safety summary

- Only the folder `/Users/Shared/WoWSyncLab` (and the project folder's `target`
  folder) is written. The scripts refuse a different folder whose name looks like
  Dropbox, World of Warcraft, Battle.net, Blizzard or a user Library.
- The only network traffic is the Rust and package download in steps 1 and 2 and
  the single GitHub request in step 4.
- The scripts stop only the one app process they started, by its own process
  number.
