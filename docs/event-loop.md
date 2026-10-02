# Event Loop Architecture

- [Overview](#overview)
- [Responsibilities](#responsibilities)
- [Design](#design)
  - [Outer Loop: parallel processing](#outer-loop-parallel-processing)
  - [Intermediate Layer: Adapters](#intermediate-layer-adapters)
  - [Inner Loop: makemkvcon](#inner-loop-makemkvcon)
- [Troubleshooting](#troubleshooting)
  - [Issues with a specific disc](#issues-with-a-specific-disc)
  - [The process never starts](#the-process-never-starts)
  - [The process runs but messages or progress are missing](#the-process-runs-but-messages-or-progress-are-missing)
  - [Progress is stale, reset, or incomplete](#progress-is-stale-reset-or-incomplete)
  - [A result disagrees with MakeMKV messages](#a-result-disagrees-with-makemkv-messages)
  - [A CLI command does not return](#a-cli-command-does-not-return)

## Overview

This page describes how makemkvcon's output reaches the CLI, where work is
scheduled, and where to start when progress or completion looks wrong. The
flow has two independent responsibilities, with a library adapter between
them:

- The **outer event loop** schedules and monitors operation tasks. This
  loop allows running multiple 'makemkvcon' invocations in parallel and
  shows progress for all tasks while those invocations are running.
- The **inner event loop** monitors one `makemkvcon` invocation, parses
  its line-based output, and emits messages and progress as they arrive.
  A run can last minutes, so callers should not wait for the process to
  finish before seeing its output.
- The **`ripit` operation layer** connects the two: each operation consumes
  MakeMKV events for one invocation, maps them to its public event type,
  and forwards them to its caller.

The complete path is shown in this diagram:

```MERMAID
flowchart LR
    cli["CLI command handler<br/>outer event loop"] --> workers["worker tasks"]
    workers --> operation["ripit operation adapter<br/>one operation per worker"]
    operation --> child["makemkvcon child process"]
    child --> readers["Concurrent stdout/stderr readers"]
    readers --> lines["Line channel<br/>capacity 256"]
    lines --> parser["run_makemkvcon<br/>ParserContext"]
    parser --> mkv_events["MakeMkvEvent channel<br/>capacity 256"]
    mkv_events --> adapter["Operation EventParser<br/>message + progress mapping"]
    adapter --> cli_events["Shared CLI event channel<br/>capacity 256"]
    cli_events --> cli

    classDef inner fill:#e8f3f1,stroke:#28766b,color:#153c37
    classDef outer fill:#f7eee3,stroke:#a9652a,color:#4a2c16
    class child,readers,lines,parser,mkv_events inner
    class cli,workers,operation,adapter,cli_events outer
```

For examples of makemkvcon's output see [makemkv-output.md](makemkv-output.md).

## Responsibilities

- **[ripit-cli](../ripit-cli)** (presentation)
  - provide a convenient console-based interface
  - handle multiple jobs in parallel and indicate progress
- **[libs/ripit](../libs/ripit)** (implementation)
  - provide a better interface for `makemkvcon` / `makemkvcon64.exe`
  - fixes issues relating to user interface
  - fixes issues relating to generated output
- **[libs/makemkv](../libs/makemkv)** (abstraction)
  - provide a Rust-based wrapper for `makemkvcon` / `makemkvcon64.exe`
  - spawns the executable and parses its output

The reason for splitting 'libs/ripit' and 'libs/makemkv' is that
`makemkvcon` (Linux) / `makemkvcon64.exe` (Windows) is an executable
provided by a third party and we have no control over its design and
future development. The interface, mode of operation and output of this
executable may change at any point in time. Providing a wrapper library
should help to contain and isolate necessary changes to this library
without affecting code in `ripit` and `ripit-cli`.

The reason for splitting 'libs/ripit' and 'ripit-cli' is that at some point
in the future there may be a need for other user interface types (e.g. a
graphical interface or a REST-based API). Splitting the "how it's done"
from the "how it's presented" allows easy integration of these alternative
interfaces should the need arise.

## Design

### Outer Loop: parallel processing

The command handlers in `ripit-cli/src/cmd_*.rs` own job scheduling and
presentation:

- `ripit-cli scan …` Reads the contents of a disc or disc image and
  provides a TEXT- or YAML-based representation.
- `ripit-cli extract …` Extracts MKV files from multiple discs or disc
  images in parallel.
- `ripit-cli unshackle …` Creates a disc images from multiple drives in
  parallel.

For multi-source commands, each task runs independently and sends operation
events to a shared bounded channel. The command loop consumes those events,
updates messages and progress, and collects completed task handles. This
allows several long-running makemkvcon processes to make progress
concurrently while the CLI reports events from each one.

The worker task, intermediate layer, makemkvcon process, and inner output
loop are repeated per active source. This is the key design split:

- per-process streaming keeps a single, long-running operation observable
- CLI-level scheduling keeps multiple operations active and their output
  distinguishable

_Please note:_ The handlers do not impose a concurrency limit.

- This may cause I/O-related starvation issues if too many image files are
  used with `ripit-cli extract`.
- This isn't a concern for `ripit-cli unshackle` because 'reading from
  optical drive' is very slow compared (~10-20 MiB/s) to 'writing to hard
  drive' (at least 100 MiB/s for 5400RPM HDDs) and it's unlikely that the
  computer has more than 2 optical drives anyway. (The maximum number of
  drives supported by `makemkvcon` is 15. Any modern SSD or NVMe would be
  able to handle that.)

### Intermediate Layer: Adapters

The functions each run one makemkvcon process and adapt its events. This is
done because makemkvcon is known to:

- fail to indicate severity; a 'MSG' record can be purely informational
  (version number) or indicate failure (drive not found, disc read error)
- fail to properly report completion (action may complete successfully
  but indicated progress does not reach 100%)
- reset 'current' and 'total' progress back to 0% for no apparent reason
- contain redundant information (backup success/failure is reported twice)
- generate events that are unrelated to the current operation (e.g. it
  makes no sense to report TCOUNT (title count) during `makemkvcon backup`
  since the content is never read and the reported number is always 0)

Providing these adapters augments the provided output with relevant
information, hide irrelevant information and normalize the data.

- [`scan`](../libs/ripit/src/scan/mod.rs) maps generic events to
  `ScanEvent` and checks the parsed title count against the reported
  count.
- [`extract`](../libs/ripit/src/extract/mod.rs) maps generic events to
  `ExtractEvent` and checks the parsed title count.
  _Its fail-severity message count is not currently used to determine the
  operation's result._
- [`unshackle_disc`](../libs/ripit/src/unshackle/mod.rs) maps generic
  events to `UnshackleEvent` and treats fail-severity messages as backup
  failure.

These adapters are responsible for handling a single source. They do not
schedule the parallel processing of multiple sources.

Each adapter spawns a makemkvcon process, receives `MakeMkvEvent`s in batches,
maps message severity and `PRGT`/`PRGC`/`PRGV` progress into the
operation-specific event type, and forwards these events to its caller.
It drains buffered events after the worker completes and then interprets
the worker result.

### Inner Loop: makemkvcon

The process runner is in [`libs/makemkv/src/runner/mod.rs`](../libs/makemkv/src/runner/mod.rs):

1. `run_makemkvcon` starts `run_program` as a Tokio task and creates a
   bounded channel with capacity 256 for output lines.
2. `run_program` launches one child process with piped stdout and stderr.
   Separate reader tasks read lines from each pipe and send them through
   the line channel. The channel carries strings, not stream labels, so the
   parser cannot guarantee the original ordering between stdout and stderr.
3. `run_makemkvcon` receives lines in batches of up to 128, writes the raw
   output to the optional log, and passes each line to
   `ParserContext::process_output`.
4. [`ParserContext`](../libs/makemkv/src/runner/parser_context/mod.rs)
   parses records and sends `MakeMkvEvent`s through a second bounded
   channel, also with capacity 256. It forwards messages, drive records,
   title counts, and progress records. Content and stream attributes are
   accumulated internally and assembled into `DiscContent` before the
   operation returns.
5. MakeMKV progress can stop short of 100% or reset unexpectedly. The
   parser filters some resets and synthesizes completion progress at
   known stage or success-message boundaries.

The runner waits for the child and its output readers, then checks its
parsed content. It currently ignores the task join result and does not
use the child exit status as the operation result. (makemkvcon's exit
code is always '0' regardless of success or failure and can be safely
ignored.)

A launch failure or incomplete output should therefore be investigated
separately from a MakeMKV message that reports failure.

_Please note:_ The batch count of 128 and 256 were chosen arbitrarily and
have no special meaning. The value should be large enough to prevent
stalling and small enough to avoid wasting memory.

## Troubleshooting

### Issues with a specific disc

`ripit-cli extract` and `ripit-cli unshackle` create a time-stamped logfile
that captures all of makemkvcon's output without modification. This file is
extremely useful to debug issues without having access to the specific disc.
Additionally this file can be processed and included in unit tests (see
[../libs/makemkv/src/runner/parser_context/mod.rs](libs/makemkv/src/runner/parser_context/mod.rs)
for an example).

### The process never starts

Check MakeMKV binary discovery in [`main.rs`](../ripit-cli/src/main.rs) and
process spawning in the runner. The current spawn path uses `expect`, and
its join error is ignored by `run_makemkvcon`.

### The process runs but messages or progress are missing

Follow the line channel and parser in the runner, then check the
operation's `EventParser` and the CLI receiver. Compare raw output
with [`makemkv-output.md`](makemkv-output.md).

### Progress is stale, reset, or incomplete

Inspect `ParserContext::process_output` for progress record filtering and
completion synthesis, then the operation adapter's progress mapping.

### A result disagrees with MakeMKV messages

Check the operation's result branch. A MakeMKV message, a parser/data
error, a title-count mismatch, and a child-process error are separate
signals and are not handled identically by every operation.

### A CLI command does not return

Inspect its worker collection and channel receive loop. In particular,
unshackle waits for an event before checking whether its worker map is
empty.

There is no explicit graceful cancellation path in the inspected loops.
Channel-send failures and several parser, process, filesystem, and
progress-renderer paths can panic.
