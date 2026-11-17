# Run 003 owner checks (D-19)

These are two small checks that only the owner can do. The answers unblock
real-data work. Neither check runs any addon code.

## Check 1 (Q-3): which `.toc` does the Forever client load?

1. A test addon was generated on the dev PC at
   `D:\WoWSyncDemo\TocProbe\WSTocProbe`. It holds four `.toc` files and no Lua
   code. Each file gives the addon a different title: `PLAIN`, `FOREVER`,
   `CAMELOT` or `VANILLA`.
2. Copy the `WSTocProbe` folder into
   `<WoW>\_forever_\Interface\AddOns\` on a PC that runs Forever.
3. Start WoW Forever. At character select, open **AddOns**.
4. Note the title shown for the probe. It reads `TocProbe <WORD>`, where
   `<WORD>` tells us which file was loaded.
5. Exit WoW and delete `WSTocProbe_<WORD>.toc` from the probe folder.
6. Repeat steps 3–5 until the probe shows `PLAIN` or disappears.
7. Report the sequence of words, for example `FOREVER → CAMELOT → PLAIN`.
8. Delete the `WSTocProbe` folder afterwards.

## Check 2 (Q-23): Dropbox online-only file

1. In `D:\Dropbox\Gaming Sync\WoW Sync Test\`, create a folder named
   `online-only-probe`.
2. Inside it, create:
   - `file.txt`, containing any text;
   - a subfolder `dir` that contains `inner.txt`.
3. Wait until Dropbox shows both as synced (green tick).
4. Right-click `file.txt`, then choose **Dropbox → Make online-only**. Do the
   same for the `dir` folder. Both should now show a cloud icon.
5. Tell the orchestrator. A worker then runs the provider tests that are
   BLOCKED today. They must not download those files.
6. Leave the probe in place until the provider tests have run. The worker
   reports when it can be deleted.

## Results

| Check | Date | Result | Recorded by |
|---|---|---|---|
| Q-3 | — | not yet run | — |
| Q-23 | — | not yet run | — |
