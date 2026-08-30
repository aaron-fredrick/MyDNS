---
name: "🐛 Bug Report"
about: Report a bug or unexpected behavior in MyDNS
title: "[BUG] "
labels: bug, triage
assignees: ''

---

## Describe the Bug
A clear and concise description of what the bug is.

## To Reproduce
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '....'
3. Scroll down to '....'
4. See error

## Expected Behavior
A clear and concise description of what you expected to happen.

## Environment Details
- **MyDNS Version**: [e.g., v0.1.1-dev or commit hash]
- **Operating System**: [e.g., Windows 11, Ubuntu 22.04, macOS 14]
- **Architecture**: [e.g., x64, ARM64]
- **Deployment**: [e.g., running from source, release binary, Docker]
- **Frontend**: [if applicable, e.g., latest, custom build]

## DNS Configuration
- **DNS Port**: [e.g., 53, 5353]
- **HTTP Port**: [e.g., 8080]
- **Resolver Mode**: [e.g., Forwarding, Recursive]
- **Resolver Priority**: [e.g., CloudflareFirst, RouterFirst]

## Logs & Output
**Relevant Logs**
Please paste any relevant terminal output or logs from the `logs/` directory here:
```text
(Paste logs here)
```

**DNS Query Example** (if applicable)
```bash
dig @127.0.0.1 -p 53 example.com A
```

**Configuration** (sanitized)
Please share your `config.toml` configuration (remove any sensitive passwords or secrets!):
```toml
# Paste sanitized config here
```

## Additional Context
- Does this happen consistently or intermittently?
- Were there any recent changes to your setup?
- Add any other context, screenshots, or network traces about the problem here.
