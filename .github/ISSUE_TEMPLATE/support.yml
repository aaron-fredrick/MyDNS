name: ❓ Question / Support
description: Ask a question or request help with configuration
title: "[Q] "
labels: ["question", "triage"]
assignees: []
body:
  - type: markdown
    attributes:
      value: |
        Thanks for reaching out! Please provide as much detail as possible so we can help you.

  - type: textarea
    id: goal
    attributes:
      label: What are you trying to achieve?
      description: A clear and concise description of your goal or what you are trying to set up.
      placeholder: What are you trying to do?
    validations:
      required: true

  - type: textarea
    id: tried
    attributes:
      label: What have you tried so far?
      description: Describe the steps you've already taken, including any commands you've run or configurations you've tried.
      placeholder: What have you already attempted?
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

  - type: textarea
    id: config
    attributes:
      label: Configuration (sanitized)
      description: Please share your `config.toml` configuration (remove any sensitive passwords or secrets!)
      render: toml
    validations:
      required: false

  - type: textarea
    id: errors
    attributes:
      label: Error Messages or Logs
      description: If you're encountering errors, please paste them here.
      render: text
    validations:
      required: false

  - type: textarea
    id: context
    attributes:
      label: Additional Context
      description: Add any other context, screenshots, or network diagrams that might help us understand your question.
    validations:
      required: false
