# hyprnav

Instead of relying on window creation order or Hyprland’s default directional logic, **hyprnav** analyses the *actual on-screen geometry* of your windows to navigate intelligently in any direction.

It works in both **tiled** and **floating** layouts, but its benefits are most visible in floating setups, large dashboards, or custom window arrangements where **true spatial navigation** is essential.

---

## Installation

### Automatic install (recommended)

```
curl -sSL https://raw.githubusercontent.com/a-lebailly/hyprnav/main/install.sh | bash
```

This downloads the latest prebuilt binary into the current directory.
To install it system-wide:

```
sudo mv ./hyprnav /usr/local/bin/hyprnav
```

---

## Build from source

**Requirements:**

* Rust

```
git clone https://github.com/a-lebailly/hyprnav.git
cd hyprnav
chmod +x build.sh
./build.sh
```

The optimized release binary will be available at:

```
dist/hyprnav
```

Install globally:

```
sudo mv dist/hyprnav /usr/local/bin/hyprnav
```

---

## Usage

```
hyprnav right     # Focus window on the right
hyprnav left      # Focus window on the left
hyprnav up        # Focus window above
hyprnav down      # Focus window below

hyprnav enable    # Enable hyprnav directional bindings in Hyprland
hyprnav disable   # Restore original movefocus bindings
hyprnav help      # Show available commands
```

---

## Hyprland configuration

### Manual bindings

Add these lines to `~/.config/hypr/hyprland.conf`:

```
bind = SUPER, right, exec, hyprnav right
bind = SUPER, left,  exec, hyprnav left
bind = SUPER, up,    exec, hyprnav up
bind = SUPER, down,  exec, hyprnav down
```

### Automatic configuration with `hyprnav enable` / `disable`

#### `hyprnav enable`

* Scans your Hyprland config (`~/.config/hypr/hyprland.conf`)
* Detects existing `movefocus` directional binds
* Replaces them with `hyprnav` equivalents
* Falls back to standard `SUPER + Arrow` if no suitable binds are found

#### `hyprnav disable`

* Removes any `hyprnav` directional binds
* Restores your original `movefocus` binds
* Falls back to standard `SUPER + Arrow` if no suitable binds are found

---

## Notes

* Works in **floating** and **tiled** layouts
* Uses real geometry from `hyprctl clients -j`
* Requires some positional overlap to determine direction
* If no suitable window exists, focus remains unchanged
