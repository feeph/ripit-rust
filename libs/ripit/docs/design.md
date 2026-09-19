# Design considerations

## async trait and 'static

The implementation uses `tokio::spawn` to execute the backup task concurrently with event monitoring:

```RUST
pub async fn unshackle_disc(
    mm: impl MakeMkvCli + std::marker::Send + 'static,
    ...
) {
    ...

    let mut th_mkv = spawn(async move {
        mm.backup(source_mkv, target_mkv_cpy, scan_mode, tx_mkv, Some(logfile)).await
    });

    ...

    loop {
        tokio::select! {
            ...
        }
    }

    ...
}
```

### Design rationale: tokio::spawn vs. tokio::task::scope

An alternative approach using `tokio::task::scope` was evaluated and rejected.

#### tokio::spawn (selected)

- Permits independent task lifecycle management separate from the event monitoring loop.
- Idiomatic for fire-and-monitor patterns where task and observer are loosely coupled.
- Requires `'static` bound on task parameters to guarantee validity beyond the function scope.
- Allows early returns without waiting for spawned tasks to complete (though current implementation doesn't use this).

#### tokio::task::scope (not selected)

- Eliminates `'static` requirement by guaranteeing reference validity through scope lifetime.
- Provides structured concurrency: all spawned tasks complete before scope exit.
- Simplifies error handling since task completion is implicit at scope boundary.
- Serializes completion: scope blocks until all tasks finish, preventing independent operation.
- Tightly couples spawned task lifecycle to calling function scope.
- Incompatible with scenarios requiring early return or dynamic task spawning.

### Re-evaluation trigger

This decision should be revisited if:

- The function needs to return early while allowing the backup task to continue in the background.
- Multiple independent backup tasks must run concurrently with shared lifetime guarantees.
- Non-`'static` references need to be passed to the task without significant refactoring.

### References

- [Structured concurrency patterns](https://docs.rs/tokio-scoped/latest/tokio_scoped/)
- [tokio::spawn documentation](https://docs.rs/tokio/latest/tokio/fn.spawn.html)
- [tokio::task::scope documentation](https://docs.rs/tokio/latest/tokio/task/fn.scope.html)
