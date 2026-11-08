# Screen helper missing-directory fix

The screen helper and quick answers previously created a child request directory with create_dir while assuming the browser-fill parent already existed. On an installation that had never initialized the optional browser helper, this failed before Codex started with Windows OS error 3. The parent was verified absent on the affected installation.

Both entry points now use one request-directory helper that creates missing parent directories, then creates a unique child. Failure messages identify directory preparation. Cleanup still removes only known request files and the unique child; sibling held/imported references are preserved.

Validation: 57 native tests passed, including fresh-install parent creation, screen/quick cleanup and sibling preservation, and contextual error reporting when the parent is a file. The actual screen-inference CLI flags and JSON schema extracted from source were run against a synthetic 2+2 PNG with the existing Codex sign-in; exit code 0 returned answer 4 and controlId null. No user screen was captured and no external app was filled during verification.
