name: 📖 Documentation
description: Suggest improvements or report issues with the documentation
title: "[DOCS] "
labels: ["documentation", "triage"]
assignees: []
body:
  - type: markdown
    attributes:
      value: |
        Thanks for helping improve the MyDNS documentation!

  - type: input
    id: file
    attributes:
      label: Documentation File
      description: Which file needs to be updated?
      placeholder: e.g., README.md, docs/production-readiness.md, docs/https-deployment.md
    validations:
      required: true

  - type: input
    id: section
    attributes:
      label: Section
      description: Which section needs to be updated?
      placeholder: e.g., Configuration section, Installation guide
    validations:
      required: false

  - type: dropdown
    id: problem_type
    attributes:
      label: What is the problem?
      description: What kind of documentation issue is this?
      options:
        - Typo or grammar error
        - Misleading or unclear information
        - Outdated instructions or examples
        - Missing information or incomplete guide
        - Broken links or references
        - Code examples don't work
        - Other
    validations:
      required: true

  - type: textarea
    id: proposed_change
    attributes:
      label: Proposed Change
      description: Describe how you think the documentation should be updated. If you have a specific rewrite in mind, please include it here.
      render: markdown
      placeholder: Paste your proposed documentation changes here
    validations:
      required: true

  - type: dropdown
    id: blocking
    attributes:
      label: Is this blocking you from using MyDNS?
      description: Does this documentation issue prevent you from using the software?
      options:
        - Yes, completely blocked
        - No, but it's confusing
        - No, just an improvement
    validations:
      required: false

  - type: textarea
    id: context
    attributes:
      label: Additional Context
      description: Have you found a workaround? Add any other context or screenshots that might help.
    validations:
      required: false
