# Security Policy

## Supported Versions

Security updates are applied to the latest release track and the `main` development branch. Users are encouraged to run the most recent version of `puls`.

| Version | Supported |
| ------- | --------- |
| 0.9.x (latest) | Yes |
| < 0.9.4 | No |

## Security Scope

`puls` is a lightweight, terminal-based system monitoring tool written in Rust. Because it reads kernel interfaces and system telemetry, we treat the following categories as security-relevant issues:

- **Privilege & System State Access:** Unauthorized or unsafe access patterns when inspecting `/proc`, `/sys`, or battery/power telemetry interfaces.
- **Buffer & Memory Safety:** Any unexpected memory safety edge cases, unsafe Rust blocks, or memory leaks occurring during continuous metric polling.
- **Terminal Escape Injection:** Unsanitized terminal control sequences or ANSI escape injection vulnerabilities originating from monitored process names or system attributes.
- **Denial of Service (DoS):** Unbounded resource consumption, CPU starvation, or deadlocks in telemetry loops.

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues, discussions, or pull requests.**

Instead, report them privately using GitHub's built-in private vulnerability reporting system:

1. Navigate to the **Security** tab of the repository: `https://github.com/word-sys/puls/security`
2. Click **Report a vulnerability** (`https://github.com/word-sys/puls/security/advisories/new`) to draft a private advisory.

### What to Include in Your Report

To help us investigate and triage the issue quickly:

- The exact version of `puls`, Rust toolchain version, and the target Linux kernel / distribution.
- Clear steps to reproduce the issue, along with relevant hardware or power interface configurations (e.g., specific `/sys/class/power_supply` topologies).
- A description of the potential impact (e.g., terminal corruption, panic/crash, unauthorized data disclosure).

### Response & Disclosure Timeline

- **Initial Acknowledgment:** Within **48 hours** of submission.
- **Triage & Assessment:** Within **5 business days**, confirming acceptance and severity.
- **Resolution & Release:** For verified vulnerabilities, a patch and regression tests will be prepared and released promptly.
- **Coordinated Disclosure:** The security advisory will be published alongside the patched release, with credit given to the reporter (unless anonymity is requested).
