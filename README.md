# Tilde

**An independent terminal focused on local use.**

Tilde focuses on the local terminal experience and does not require an account. Legacy cloud/AI
source and dependencies are still being removed. GitHub Releases updates and necessary local
networking remain; this is not a verified claim of network silence. See the
[local-only maintenance policy and audit](LOCAL_ONLY_MAINTENANCE.md) for the removal ledger,
upstream backport allowlist, dependency guard, and remaining work.

Tilde is an independent derivative of the
[open-source Warp client](https://github.com/warpdotdev/warp). It is not affiliated with, endorsed
by, or maintained by Warp or Denver Technologies, Inc.

## Development

```bash
./script/bootstrap
./script/run
./script/run-tui
./script/presubmit
```

## Source and issues

- Source: <https://github.com/nguyenphutrong/tilde>
- Issues: <https://github.com/nguyenphutrong/tilde/issues>

## Licensing

Most of Tilde is licensed under the [GNU Affero General Public License v3](LICENSE-AGPL).
The `warpui` and `warpui_core` crates retain their [MIT license](LICENSE-MIT).

Tilde contains modifications to the Warp client. Original and third-party attribution is recorded
in [NOTICE](NOTICE) and the license files shipped with each distribution.
