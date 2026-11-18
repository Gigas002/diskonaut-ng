# Audit

A whole-application audit of diskonaut and libdiskonaut (2026-11-18) in five
parts: bugs (§2), performance (§3), security and robustness (§4), usability
(§5), and code quality, tests and project hygiene (§6). §7 lists what was left
open, and §8 is the checklist to re-run the audit.

Each finding says what was found, how serious it is, and what was done:
**fixed** (with the commit), **accepted** (with the reason), or **OK**
(checked, nothing to do). Every fix is its own commit, with a regression test
where one can be written.

Severity: **high** (wrong sizes, a crash in normal use, or a delete that
can't be done), **medium** (wrong behavior in a reachable case, an ignored
setting), **low** (hardening, polish, or needs an unusual setup).

## 1. Method

Every source file was read in full. Findings were then reproduced in three
ways:

- **Real scans** of `~/.cargo/registry` (116k entries), `/usr/lib/git-core`
  (hard links) and `/usr` (418k entries), compared with `du -sB1`.
- **Drawing sweeps** with ratatui's `TestBackend`: every UI mode at 150
  terminal sizes, with every tile selected in turn, for names in Cyrillic,
  CJK, emoji and Latin-1 bytes that aren't UTF-8.
- **The release binary in a pty**, which answers the terminal's cursor
  position queries like an emulator does: start-up errors, a pipe, deleting
  (including a non-UTF-8 name) and quitting, with the tty's `icanon`/`echo`
  checked after exit.

Builds checked: default, `--no-default-features`, `--all-features`. Static
checks: `cargo clippy` at the three gate levels, `cargo deny check`, `typos`,
`cargo llvm-cov`.

## 2. Bugs

| Area | Finding | Status |
| ---- | ------- | ------ |
| Folders emptied during the scan | The parallel walk can report a folder after entries inside it. `add_folder` then replaced the folder with an empty one: 744 folders of a `~/.cargo/registry` scan lost their files, which were missing from the treemap and couldn't be deleted, while their size and count stayed. Later deletions in them could underflow the counts. 0 inconsistent folders after the fix over three scans, and the total matches `du`. | Fixed, high (ca099b8) |
| Hard links | Every link added the file's size: three links to a 64K file made 192K, and `/usr/lib/git-core` showed 10977280 bytes against `du`'s 10870784. Like `du`, a file is now counted once per (device, inode). The other links are listed with no size, so deleting one doesn't claim space it can't free. `/usr/lib/git-core` now matches `du` exactly. | Fixed, high (7109d3e) |
| Non-UTF-8 names | The delete prompt and the title converted paths with `into_string().expect(...)`. Pressing `d` on a Latin-1 name, or starting diskonaut in such a folder, crashed it. Names are shown lossily now; checked with the binary. | Fixed, high (5523f8b) |
| Text widths | Widths were byte lengths. The delete prompt subtracted a name's bytes from the box width, which panicked in debug builds and misplaced the line in release. `truncate_middle` shortened names that fit. `truncate_end` cut inside a character, so an error message with non-ASCII text crashed the error box. The title telescope placed its cells by bytes. The sweep found 319 panics in debug builds and 223 in release; there are none now. | Fixed, medium (5523f8b) |
| Terminal left raw | Raw mode was set before the folder was resolved, so `diskonaut /nonexistent` exited with the shell at `-icanon -echo`. Nothing restored the terminal on a panic. Raw mode is now a drop guard, set after the folder check, and a panic hook restores the terminal. Errors go to stderr. | Fixed, medium (43e5746) |
| Piped output | `diskonaut \| less` waited for a cursor-position reply that never came, then panicked ("failed to clear terminal") and left the terminal raw. The `NoStdout` error existed but was never returned; stdout must now be a terminal. | Fixed, low (43e5746) |
| Shifted keybinds | Terminals report `D` and `?` with Shift held, and bindings compared modifiers exactly, so `delete = "D"` or `quit = "?"` never fired; `shift+a` didn't either. `Ctrl+C` written with capitals didn't parse. | Fixed, medium (8229ea9) |
| Unknown config keys | `apparent_size` for `apparent-size`, or a misspelled action, did nothing, silently. These are now errors that name the field. | Fixed, medium (8229ea9) |
| Wrong key hints | The controls line said `<BACKSPACE> - delete`, but delete has been `d` since 0.12.2. Every hint (controls, `(y/n)`, `<ESC> to dismiss`) ignored the configured keys. | Fixed, medium (e7a3d32) |
| Dangling symlinks | Deleting one failed with "No such file or directory", because the type was read through the link. | Fixed, low (7eec2e4) |
| Delete during the scan | `d` shows a warning until the scan is done, so a deleted folder can't be re-added by entries still in flight. | OK |
| Empty folders | A folder of empty files (0/0) shares the area equally; there is no NaN in the sort. | OK |

## 3. Performance

| Area | Finding | Status |
| ---- | ------- | ------ |
| Scan | `/usr` (418k entries) scans in 1.2–1.4 s with 127 MB peak RSS (about 300 bytes an entry), using all cores through dua-core. Totals match `du`. | Accepted |
| Treemap | Laying out a folder of 200k files takes 88 ms. Recursion is bounded by the number of tiles that can be drawn. | Accepted |
| Mounts | Scanning `/` walked `/proc`, `/sys`, other disks and network mounts; with `-a`, `/proc/kcore` counted as 128T. Crossing stays the default, as in `du`, `ncdu` and `dua`, and `-x` / `[base] one-file-system` stays on one filesystem. | Fixed, medium (85e0265) |

## 4. Security and robustness

| Area | Finding | Status |
| ---- | ------- | ------ |
| Symlinks | The walk never follows them. Deleting a link to a folder or a file removes only the link; a test checks that the target stays. | OK, test (7eec2e4) |
| Deleting | It is permanent (`remove_dir_all`), after a `y/n` prompt that names the item and, for a folder, its file count. | Accepted, see §7 |
| Input thread | `read().unwrap()` in the terminal event iterator panics only when the terminal is gone, and then SIGHUP has already ended the process. The panic hook restores the terminal in any case. | Accepted |
| Dependencies | `cargo deny check` passes and the lockfile is current. | OK |

## 5. Usability

| Area | Finding | Status |
| ---- | ------- | ------ |
| Long names | A selected name too long for the bottom line wasn't shown at all; it is now shortened in the middle. | Fixed (5523f8b) |
| Config docs | `example/config.toml` explains how characters match and that unknown keys are errors; the README and example document `-x`. The example config is tested to load and to match the defaults. | Fixed, docs (8229ea9, 85e0265) |

## 6. Code quality, tests and hygiene

| Area | Finding | Status |
| ---- | ------- | ------ |
| Shared temp paths | Tests wrote to fixed names in `/tmp`, so parallel runs (two checkouts, or CI jobs) shared and deleted each other's directories. Each test has its own `tempfile` directory now. | Fixed (860a4d7) |
| Wall-clock test | The event test slept 600 ms and assumed the 250 ms flash was over. It now blocks on the instructions in order. | Fixed (d9978f4) |
| Untested input | Key handling and the instruction loop had little or no coverage (32 % and 0 %). A test queues a scan and a key script into the real loop and checks the end state, with no waiting on time. The new tests passed 24 parallel runs. | Fixed (d9978f4) |
| Coverage | 58.1 % → 85.4 % of lines (`cargo llvm-cov --workspace --all-features`). `main.rs` (0 %) runs only as the real binary and was checked in a pty. | Fixed |
| Cargo warnings | `package.homepage` repeated `package.repository` and warned on every build. | Fixed (3a6bc4d) |
| Gates | fmt, typos, deny, clippy and tests at the three feature levels, and docs, pass on every commit. | OK |

## 7. Left open

- **Trash instead of delete.** Moving to the freedesktop trash (`trash` crate)
  would make a mistaken `y` recoverable, at the cost of not freeing the space
  until the trash is emptied, which is often what someone running diskonaut
  wants. That's a product decision, not a bug.
- **`ScanOptions` is a plain struct.** Adding `one_file_system` broke code
  that builds it field by field. `#[non_exhaustive]` with a builder would
  avoid that next time; libdiskonaut is pre-1.0, so it is only noted in the
  changelog.
- **Test directories on failure.** Test helpers `keep()` their temp
  directories, so a failing test leaves its directory in `/tmp`. Returning
  the `TempDir` guard would clean up on panic too.
- **`main.rs` coverage.** It could take the terminal as a parameter and be
  tested like ratgreet's, at the cost of a slimmer `main`.

## 8. Re-running the audit

```sh
# Gates (docs/ARCHITECTURE.md §8)
cargo fmt --all -- --check && typos && cargo deny check
for f in --no-default-features "" --all-features; do
  cargo clippy --workspace --all-targets $f -- -D warnings
  cargo test --workspace $f
done
cargo doc --workspace --no-deps

# Coverage (system LLVM tools on Arch: LLVM_COV / LLVM_PROFDATA)
LLVM_COV=/usr/bin/llvm-cov LLVM_PROFDATA=/usr/bin/llvm-profdata \
  cargo llvm-cov --workspace --all-features --summary-only

# Totals against du (should be equal; add -x to both for one filesystem)
du -sB1 /usr/lib/git-core
```

To compare a total by hand, scan a folder with `libdiskonaut::scan_into_tree`
and print `get_total_size()`. Use `ScanOptions { show_apparent_size: true, .. }`
to compare with `du -sb`.
