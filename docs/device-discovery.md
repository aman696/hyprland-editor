# Detected device names

Open **Edit Devices → Device Name and Category → Detected Devices**, then choose **Refresh Devices**. The editor reads names and categories from the selected Hyprland session on demand, in the background. Refresh leaves the explicit prompt selected and does not modify the match form or draft.

Selecting a named keyboard, pointer, tablet or touch record fills the existing name/category controls. **Update Device Match** is still required to keep that match in the draft; it uses ordinary validation, history and undo. The category is a field-relevance hint. Pointer records may include touchpads and virtual devices; their names are not used to guess a physical subtype. Choose the appropriate category manually when needed. No device settings or active keymap are changed.

Tablet pads/tools can lack an independent configuration name. A pad's reported parent is shown as context and never substituted for the pad's identity. Switch records are visible as observations and are not offered as device-override targets. Selecting either type leaves the existing match form unchanged and explains why. Empty/missing names and unrecognized roles also remain observations. Duplicate names across records/categories are preserved, not silently merged. Reported keyboard layout/keymap text is informational and is not copied into configuration fields.

## Transport and interpretation

The closed read-only request enum adds `j/devices`. Each snapshot uses exactly `j/version` followed by `j/devices` through the existing pinned `ReadSession`, with process/socket/peer identity checks and rechecks, two-second absolute request deadlines and a 2 MiB reply limit. No shell, arbitrary request, setter, input-event reader or external `hyprctl` process is involved.

The decoder keeps the five documented response categories: mice, keyboards, tablets, touch and switches. It accepts at most 512 records in total and limits each retained string to 4 KiB. Control characters in device names, duplicate known JSON keys, invalid shapes and malformed/oversized responses are refused. Optional values remain unknown; missing known categories are counted separately from reported empty lists. Unknown response fields/categories are ignored rather than classified by guesswork. Addresses are not retained as stable hardware identities.

The pinned [Hyprland device request implementation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/debug/HyprCtl.cpp#L694) defines those categories and the differing pad/tool records. It can emit a bare `none` for an unavailable keyboard layout index, which is invalid JSON. Such a response produces a visible discovery error; the editor does not repair arbitrary JSON heuristically or invent a layout index. A compositor build without an exact audited commit match remains explicitly unmapped.

Results are window-local, discarded when their editor closes or the draft generation/version changes during the request, and never restored as current hardware observations from a saved project. No idle polling/service is added. The existing background completion helper runs only while the requested operation is in flight. Manual observations can become stale after a request completes.

## Evidence and remaining work

Private Unix-socket fixtures verify the exact two read requests, unmapped builds, missing categories, duplicate names, pad/owner separation, switch handling and malformed/oversized reply refusal. The private Xvfb/DBus/temporary-XDG graphical fixture exercises prompt-preserving refresh, form-only selection, the real match action and undo, non-target records, no subtype inference from names, narrow layout, stale/closed-window result disposal and unavailable discovery. It does not connect to the user's desktop.

This is name/category discovery, not complete effective input configuration or a hardware certificate. Multi-session selection, hotplug refresh, capability/physical dimension records, effective global/default/override provenance, actual keymap compilation/preview, switch binding workflows, last-input recovery, live apply, representative resource/accessibility tests and full device acceptance remain required.
