name: 🐛 Bug Report
description: Report a bug or unexpected behavior in MyDNS
title: "[BUG] "
labels: ["bug", "triage"]
assignees: []
body:
  - type: markdown
    attributes:
      value: |
        Thanks for taking the time to fill out this bug report!

  - type: textarea
    id: description
    attributes:
      label: Describe the Bug
      description: A clear and concise description of what the bug is.
      placeholder: What happened?
    validations:
      required: true

  - type: textarea
    id: reproduce
    attributes:
      label: To Reproduce
      description: Steps to reproduce the behavior
      placeholder: |
        1. Go to '...'
        2. Click on '....'
        3. Scroll down to '....'
        4. See error
    validations:
      required: true

  - type: textarea
    id: expected
    attributes:
      label: Expected Behavior
      description: A clear and concise description of what you expected to happen.
      placeholder: What should have happened?
    validations:
      required: true

  - type: dropdown
    id: os
    attributes:
      label: Operating System
      description: What operating system are you using?
      options:
        - Windows 11
        - Windows 10
        - Ubuntu 22.04
        - Ubuntu 20.04
        - Debian 12
        - macOS 14 (Sonoma)
        - macOS 13 (Ventura)
        - Other
    validations:
      required: true

  - type: input
    id: version
    attributes:
      label: MyDNS Version
      description: What version of MyDNS are you using? (e.g., v0.1.1-dev or commit hash)
      placeholder: v0.1.1-dev
    validations:
      required: true

  - type: dropdown
    id: deployment
    attributes:
      label: Deployment Method
      description: How are you running MyDNS?
      options:
        - Running from source
        - Release binary
        - Docker
        - Other
    validations:
      required: true

  - type: input
    id: dns_port
    attributes:
      label: DNS Port
      description: What port is the DNS server listening on?
      placeholder: 53
    validations:
      required: false

  - type: input
    id: http_port
    attributes:
      label: HTTP Port
      description: What port is the HTTP server listening on?
      placeholder: 8080
    validations:
      required: false

  - type: dropdown
    id: resolver_mode
    attributes:
      label: Resolver Mode
      description: What resolver mode are you using?
      options:
        - Forwarding
        - Recursive
        - Other
    validations:
      required: false

  - type: textarea
    id: logs
    attributes:
      label: Relevant Logs
      description: Please paste any relevant terminal output or logs from the `logs/` directory
      render: text
    validations:
      required: false

  - type: textarea
    id: config
    attributes:
      label: Configuration (sanitized)
      description: Please share your `config.toml` configuration (remove any sensitive passwords or secrets!)
      render: toml
    validations:
      required: false

  - type: textarea
    id: context
    attributes:
      label: Additional Context
      description: Does this happen consistently or intermittently? Were there any recent changes to your setup?
    validations:
      required: false
