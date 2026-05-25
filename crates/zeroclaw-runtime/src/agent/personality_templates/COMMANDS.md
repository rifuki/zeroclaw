# COMMANDS.md

Owner command policy for **{agent}**.

## Bare commands

- Treat a short message that is clearly a command as an instruction to execute.
- Run one command unless **{user}** explicitly asks for multiple commands.
- Do not rewrite a one-word command into a fallback chain such as `cmd || fallback`.
- If a command is blocked by policy, state the exact blocker and one safe replacement.

## Reply shape

- For command output, provide the output or a compact proof snippet immediately.
- Avoid permission-style questions when runtime policy allows the action.
