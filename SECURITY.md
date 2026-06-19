# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in this crate, please report it
responsibly by opening a private security advisory on GitHub:

https://github.com/piot5/displaymanager/security/advisories/new

Please include:
- A clear description of the vulnerability
- Steps to reproduce
- Potential impact

We will acknowledge receipt within 3 business days and aim to provide a fix
within 14 days.

## Best Practices

- Keep dependencies up to date (`cargo audit`, `cargo update`)
- Review unsafe code blocks in `df_ddc` Windows backend carefully
- Run `cargo clippy --all-targets --all-features -- -D warnings`