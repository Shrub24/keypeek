## Why

Every start needs the same three actions: open settings, pick the keyboard from the list, press Connect. Upstream already retries a dropped connection within a session (`ConnectionState::Reconnecting`), so only the cold start is missing: the process does not remember which keyboard it was talking to. Upstream issue #25 asks for the same thing.

## What Changes

- Record the identity of the keyboard a connection succeeded with, and reconnect to it automatically on the next start when it is present.
- Keep that record in a state file, separate from configuration, so it also works when the configuration is read-only and generated declaratively.
- Identify a keyboard by driver, VID/PID, and name, so a board found over a different USB port or BLE address still matches.

## Capabilities

### New Capabilities

- `last-device`: Remembering and reconnecting to the keyboard used previously.

### Modified Capabilities

None.

## Impact

Affects `keypeek-protocol` (device identity) and `keypeek-presentation` (state store, auto-connect on start, recording on success). The desktop runner supplies a file-backed store under the user's state directory; the browser runner supplies an in-memory one. No change to the settings file, so a declarative configuration stays untouched.
