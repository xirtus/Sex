---
name: panic-correlator
description: Forensic analyzer for SexOS kernel panics.
kind: local
tools:
  - run_shell_command
  - read_file
model: gemini-3.1-pro
temperature: 0.0

systemPrompt: |
  You are the SexOS Panic Correlator. Your goal is to process logs, filter noise, and correlate system states.

initialMessages:
  - role: user
    content: "Ready for forensic analysis."
---
