# KeyPeek <img src="resources/icon.svg" align="right" width="15%"/>

<a href="https://ko-fi.com/stephanrwi"><img src="https://img.shields.io/static/v1?label=Buy%20me%20a%20coffee&message=%E2%98%95&logo=Ko-fi&color=%23FF5E5B"/></a>
<a href="https://paypal.me/rumswinkel"><img src="https://img.shields.io/static/v1?label=Donate&message=%E2%9D%A4&logo=PayPal&color=%23009cde"/></a>

KeyPeek provides real-time layout visualization and an integrated keymap editor for QMK, Vial, and ZMK keyboards, running in two ways:

- **Desktop app:** Designed for active daily use. A live on-screen overlay that mirrors base and momentary layers in real time (especially useful when learning complex layouts or using boards with missing legends), with the ability to tweak keys directly without interrupting your workflow.
- **Web app:** A zero-install browser editor (USB and bluetooth) focused on inspecting layouts and remapping keys, sending keycode and behavior updates to the device on the fly from any computer.

<p align="center">
  <a href="https://github.com/srwi/keypeek/releases"><strong>Get desktop app</strong></a> &nbsp;|&nbsp; <a href="https://srwi.github.io/keypeek/"><strong>Open web app</strong></a>
</p>

<img src=".github/assets/demo.gif" alt="KeyPeek in action">

## Setup

Using the desktop overlay requires a small firmware change because stock QMK, Vial, and ZMK firmware do not expose live layer-change events. The module adds this event stream over the device connection so the overlay stays in sync with your active layers in real time while typing.

> [!NOTE]
> If you only plan to use KeyPeek for keymap editing, this firmware modification is not needed.

### QMK and Vial

1. In your QMK userspace (or `qmk_firmware`) root, add the module repo:
   
   ```sh
   mkdir -p modules
   git submodule add https://github.com/srwi/qmk-modules.git modules/srwi
   git submodule update --init --recursive
   ```
   
2. In your keymap folder, add `srwi/keypeek_layer_notify` to `keymap.json`:
   
   ```json
   {
     "modules": [
       "srwi/keypeek_layer_notify"
     ]
   }
   ```
   
3. In the same keymap folder, enable RAW HID and VIA in `rules.mk`:
   
   ```make
   RAW_ENABLE = yes
   VIA_ENABLE = yes
   ```
   
4. Build and flash your firmware:
   
   ```sh
   qmk compile -kb <your_keyboard> -km <your_keymap>
   ```
   
5. **QMK only:** Export layout information to `keyboard_info.json`:
   
   ```sh
   qmk info -kb <your_keyboard> -m -f json > keyboard_info.json
   ```
   
   This last step is only required for QMK keyboards, because VIA does not provide physical layout data directly over the connection. Vial keyboards do not require this step, as the layout data is transmitted when connecting the keyboard to KeyPeek.

### ZMK

1. Add the KeyPeek module to your `zmk-config/config/west.yml`:

   ```yaml
   manifest:
     remotes:
       - name: zmkfirmware
         url-base: https://github.com/zmkfirmware
       - name: zzeneg # <-- required for Raw HID module
         url-base: https://github.com/zzeneg
       - name: srwi # <-- required for KeyPeek module
         url-base: https://github.com/srwi
     projects:
       - name: zmk
         remote: zmkfirmware
         revision: main
         import: app/west.yml
       - name: zmk-raw-hid # <-- Raw HID module
         remote: zzeneg
         revision: main
       - name: zmk-keypeek-layer-notifier # <-- KeyPeek module
         remote: srwi
         revision: master
   ```

2. Add the `raw_hid_adapter` as an additional shield to your build, e.g. in `build.yaml`:
   
   ```yaml
   include:
     - board: nice_nano_v2
       shield: <existing shields> raw_hid_adapter # <-- required for Raw HID support
       snippet: studio-rpc-usb-uart # <-- required for ZMK Studio support
   ```
   
   **Note:** If you are using a split keyboard, the change above is only required for the central half.

3. Enable ZMK Studio support in your `.conf` file:
   
   ```conf
   CONFIG_ZMK_STUDIO=y
   ```
   
   If your keyboard does not support ZMK Studio yet, adding support is described in the [ZMK documentation](https://zmk.dev/docs/features/studio#adding-zmk-studio-support-to-a-keyboard).

KeyPeek will read layout and keymap directly from the device for ZMK without requiring additional configuration.

> [!NOTE]
> If the keyboard has been paired via Bluetooth before enabling raw HID support, re-pairing may be necessary to allow the new communication channel.

### Linux permissions

USB access is covered by the ZMK udev rules that ship with most distributions: devices with USB ID `1d50:615e` (the default ZMK identity, used by boards such as the nice!nano) need write access to their serial port and HID interfaces, which those rules grant.

BLE keyboards have no USB identity at all, so those rules cannot match them. For BLE HID devices, access is granted by modalias instead. The Nix package includes a rule for ZMK's default identity at `lib/udev/rules.d/60-keypeek.rules`:

```
KERNEL=="hidraw*", ATTRS{modalias}=="hid:b0005g0001v00001D50p0000615E", TAG+="uaccess"
```

On NixOS, add the package to `services.udev.packages`. On other distributions with system-manager, the `systemManagerModules.default` flake output links the packaged rule into `/etc/udev/rules.d`; on any other system, copy the rule from the package into `/etc/udev/rules.d/` manually. Rules are only evaluated when a device appears, so unplug and replug the keyboard (or run `udevadm control --reload && udevadm trigger`) after installing or changing a rule.

### Declarative setup (Nix)

The flake exports `homeManagerModules.default` with a `programs.keypeek` option set:

```nix
programs.keypeek = {
  enable = true;
  settings = {
    size = 80;
    position = "Bottom Right";
    offset_x = 40;
    offset_y = 12;
    monitor = "DP-2";
  };
  service.enable = true;
};
```

`settings` become the `[settings]` section of a generated INI file that only the bundled systemd user service reads, via `--config`. That mode is read-only: KeyPeek never writes the file back, so a settings window opened from the launcher changes the running instance for that session only. Setting `settings` without `service.enable` does nothing; the module warns about it.

The service starts with `--no-settings`, so it never opens a window on its own. It needs the session environment to reach the compositor; `uwsm` and most compositor setups export it into the systemd user manager. If yours does not, name the variables:

```nix
programs.keypeek.service.environment = { WAYLAND_DISPLAY = "wayland-1"; };
```

## Usage

<img src=".github/assets/edit_key_window.png" alt="Key editor screenshot" width="100%">

### Connecting

- **Desktop:** Devices are scanned when the app starts. For QMK you will be prompted to select the `keyboard_info.json` generated from your keymap when you connect. For Vial and ZMK, just select the connected device from the dropdown, since they provide layout information directly.
- **Web:** In a browser supporting WebHID, Web Serial, or Web Bluetooth (such as Chrome or Edge), select your keyboard using the browser pairing dialog.

### Remembered keyboard

The desktop app remembers the keyboard it last connected to and reconnects to it automatically on the next start, if it is present. It is identified by driver, vendor/product ID, and name, so it is still recognised when it moves to another USB port or connects over Bluetooth instead.

The record lives in `~/.local/state/keypeek/last-device` (the `$XDG_STATE_HOME` equivalent), not in `settings.ini` — it is state, not configuration, so a declarative setup is never rewritten. Delete that file to start disconnected again. If the remembered keyboard is not present, KeyPeek starts disconnected rather than picking a different device.

### Overlay visibility

By default the overlay follows the layers you hold and disappears after the configured timeout. A running instance can also be driven from outside:

```sh
keypeek --toggle     # hide if visible, show if hidden
keypeek --show       # pin visible, regardless of layers
keypeek --hide       # hide until re-enabled
keypeek --auto       # back to layer-driven visibility
keypeek --settings   # open the settings window
```

Bind them like any other command in your compositor's configuration, for example `bind = SUPER, K, exec, keypeek --toggle` in a wlroots-style compositor.

These reach the running instance over `$XDG_RUNTIME_DIR/keypeek.sock`. The hotkey commands do nothing when KeyPeek is not running; `--settings` is the exception — it starts an instance with the settings window open when none is listening, which is why the desktop entry uses it. Launching KeyPeek from your application menu therefore configures the running instance instead of starting a second overlay.

With a declarative configuration the running instance cannot write settings (the file comes from the Nix store), so changes made in a window opened this way last for the session. Persistent changes belong in the Nix configuration.

When KeyPeek is started by the bundled systemd service, `--no-settings` keeps the settings window closed; a normal start opens it only when no remembered keyboard was connected.

The override lasts until it is changed again and is not written to `settings.ini`, so a declarative configuration stays authoritative. A settings or key editor window always keeps the overlay visible, because the editor selects keys by clicking them.

On a multi-monitor setup, choose the display the overlay appears on under *Alignment → Monitor*. The choice is stored by output name, so it survives restarts and display renumbering; `Primary` leaves the choice to the display server. Changing it moves the running overlay immediately.

This applies to native Wayland sessions, where the overlay is a layer surface. On X11 the overlay stays on the primary display and the settings window says that output names are unavailable.

### Placement

The overlay is placed by an *Alignment* anchor (a corner, an edge midpoint, or the centre) plus two offsets, measured inward from the edge that anchor uses on each axis. A negative offset moves the overlay past that edge, which is how you tuck it half off-screen.

```ini
position = BottomRight
offset_x = 40
offset_y = 12
```

Each axis is independent: 40 px in from the right and 12 px up from the bottom. An older file with a single `margin` key still loads, and applies that value to both offsets.

### Keymap Editor

While the settings window is open, clicking any key on the overlay opens the key editor. From there keycodes and behaviors can be selected from categorized lists or found using the search bar. You can switch between layers using the buttons at the top of the editor or click another key on the overlay to change the selection. Key changes are sent directly to the keyboard over the connection.

## Project documentation

- [Architecture](docs/architecture.md)
- [Technology stack](docs/technology-stack.md)
- [Development standards](docs/development.md)
- [Roadmap and external-change policy](docs/roadmap.md)

# Support

If you're enjoying KeyPeek, I'd be truly grateful for your support!
You can show your appreciation by giving a star on GitHub or making a donation:

- **[Buy me a coffee](https://ko-fi.com/stephanrwi)**
- **[Donate via PayPal](https://paypal.me/rumswinkel)**

# License & Attribution

Parts of this project are based on code from [the VIA project](https://github.com/the-via/app), which is licensed under the GNU General Public License v3.0.
