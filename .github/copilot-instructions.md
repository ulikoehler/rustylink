## Validation

Before finishing your tasks, you must **always** run the following commands to ensure that your changes do not break anything.s:

```sh
cargo check --all-features
```

The command might take up to 5 minutes to complete. **Wait for it to complete.**

**Important:** You need to watch for "error" messages in the output and check the return code!

After that, run all tests with all features enabled:

```sh
cargo test --all-features
```

**Important:** You need to watch for "error" messages in the output and check the return code!

If there are any errors, fix them before submitting your changes.