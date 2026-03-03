---
name: codereview
description: Thorough code review for quality, security, and best practices. Can review a single file, directory, or multiple repos in parallel.
context: fork
agent: Explore
argument-hint: [path or directory]
---

## Code Review

Review the code at `$ARGUMENTS` (defaults to the current project root if no argument given).

### If given a directory containing multiple repos/projects:

Launch a parallel review for each one. Summarize findings per-repo at the end.

### What to review:

**Bugs & Correctness**
- Logic errors, off-by-ones, null/undefined risks
- Unhandled error paths, missing edge cases
- Race conditions or concurrency issues

**Security**
- Injection vulnerabilities (SQL, XSS, command)
- Secrets or credentials in source
- Unsafe deserialization, missing input validation

**Performance**
- N+1 queries, unnecessary allocations
- Missing indexes or expensive loops
- Memory leaks or unbounded growth

**Code Quality**
- Dead code, unused imports, copy-paste duplication
- Functions that are too long or do too many things
- Naming clarity and consistency

**Architecture**
- Separation of concerns
- Dependency direction (are abstractions leaking?)
- Test coverage gaps

### Output format:

For each file with findings, report:

```
### path/to/file.ext

- **[severity]** line N: description of issue
  Suggestion: how to fix
```

Severity levels: `critical` | `warning` | `info`

End with a summary table:

| Severity | Count |
|----------|-------|
| Critical | N     |
| Warning  | N     |
| Info     | N     |

If the code looks good, say so — don't invent problems.
